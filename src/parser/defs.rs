use super::{Parser, PendingDerives};
use crate::ast::*;
use crate::intern::{FxMap, Name};
use crate::names;
use crate::source::Span;
use crate::token::Tok;
use crate::source_version::Migration;

fn is_def_keyword(k: Tok) -> bool {
    matches!(
        k,
        Tok::KwVal | Tok::KwVar | Tok::KwDef | Tok::KwClass | Tok::KwTrait | Tok::KwObject
            | Tok::KwEnum | Tok::KwType | Tok::KwGiven
    )
}

/// The modifiers written in source have the low bits of `mods`.
const MODIFIER_BITS: usize = 22;

fn modifier_word(bit: Mods) -> &'static str {
    match bit {
        mods::PRIVATE => "private",
        mods::PROTECTED => "protected",
        mods::SEALED => "sealed",
        mods::ABSTRACT => "abstract",
        mods::FINAL => "final",
        mods::CASE => "case",
        mods::LAZY => "lazy",
        mods::OVERRIDE => "override",
        mods::OPAQUE => "opaque",
        mods::INLINE => "inline",
        mods::TRANSPARENT => "transparent",
        mods::OPEN => "open",
        mods::INFIX => "infix",
        mods::IMPLICIT => "implicit",
        _ => "?",
    }
}

fn is_hard_modifier(k: Tok) -> bool {
    matches!(
        k,
        Tok::KwPrivate | Tok::KwProtected | Tok::KwSealed | Tok::KwAbstract | Tok::KwFinal
            | Tok::KwLazy | Tok::KwOverride | Tok::KwImplicit
    )
}

impl<'a> Parser<'a> {
    fn soft_modifier_at(&self, n: usize) -> Option<Mods> {
        let t = self.toks.get(self.pos + n)?;
        if t.kind != Tok::Ident {
            return None;
        }
        let bit = match t.name {
            names::OPAQUE => mods::OPAQUE,
            names::INLINE => mods::INLINE,
            names::TRANSPARENT => mods::TRANSPARENT,
            names::OPEN => mods::OPEN,
            names::INFIX => mods::INFIX,
            names::ERASED => 0,
            _ => return None,
        };
        let next = self.kind_at(n + 1);
        let follows = is_def_keyword(next)
            || is_hard_modifier(next)
            || next == Tok::KwCase
            || self.soft_modifier_at(n + 1).is_some();
        follows.then_some(bit)
    }

    /// `erased` belongs to the experimental capture and erasure features and is left out.
    pub(super) fn reject_macro_modifier(&mut self) {
        if self.tok().name == names::ERASED {
            self.error_at(self.span(), "'erased' is not part of the supported Scala subset");
        }
    }

    pub(super) fn at_def_start(&self) -> bool {
        let k = self.kind();
        if is_def_keyword(k) || is_hard_modifier(k) || k == Tok::At {
            return true;
        }
        match k {
            Tok::KwCase => matches!(self.kind_at(1), Tok::KwClass | Tok::KwObject),
            Tok::Ident if self.tok().name == names::EXTENSION => {
                matches!(self.kind_at(1), Tok::LParen | Tok::LBracket)
            }
            Tok::Ident => self.soft_modifier_at(0).is_some(),
            _ => false,
        }
    }

    /// True at `(` when the parentheses hold nothing but comma separated string literals.
    fn string_args_follow(&self) -> bool {
        let mut i = 1;
        loop {
            if self.kind_at(i) != Tok::StringLit {
                return false;
            }
            match self.kind_at(i + 1) {
                Tok::RParen => return true,
                Tok::Comma => i += 2,
                _ => return false,
            }
        }
    }

    /// `pkg.Obj.Name` as a type, with its last name.
    fn parse_class_path(&mut self) -> (TyExprId, Name) {
        let (mut name, span) = self.expect_ident();
        let mut path = self.ast.add_ty(TyExpr::Name(name), span);
        while self.at(Tok::Dot) && self.kind_at(1) == Tok::Ident {
            self.bump();
            let segment = self.bump();
            name = segment.name;
            path = self.ast.add_ty(TyExpr::Select(path, name), span.to(segment.span));
            self.note_ty_name(path, segment.span);
        }
        (path, name)
    }

    /// `@name`, `@path.name[T](args)(more)`, kept as the `new` it stands for. String literal
    /// arguments are also kept as such for the annotations the compiler reads itself.
    pub(super) fn parse_annots(&mut self) -> Vec<Annot> {
        let mut annots = Vec::new();
        while self.at(Tok::At) {
            let start = self.bump().span;
            let (mut path, name) = self.parse_class_path();
            if self.at(Tok::LBracket) {
                // `simpleType`'s applied type, from the path (the `@` is the annotation's).
                let from = self.ast.ty_spans[path.idx()];
                let targs = self.parse_type_args();
                path = self.ast.add_ty(TyExpr::Apply(path, targs), from.to(self.prev_span()));
            }
            let mut args = ListRef::EMPTY;
            let mut values = ListRef::EMPTY;
            if self.at(Tok::LParen) {
                if self.string_args_follow() {
                    self.bump();
                    let start = self.ast.str_lists.len() as u32;
                    let mark = self.expr_scratch.len();
                    while self.at(Tok::StringLit) {
                        let lit = self.parse_literal();
                        if let Expr::StringLit(s) = self.ast.expr(lit) {
                            self.ast.str_lists.push(s);
                        }
                        self.expr_scratch.push(lit);
                        self.eat(Tok::Comma);
                    }
                    self.expect(Tok::RParen);
                    args = ListRef { start, len: self.ast.str_lists.len() as u32 - start };
                    values = self.expr_list(mark);
                } else {
                    values = self.parse_args().0;
                }
            }
            let mut instance = self.ast.add_expr(Expr::New(path, values), start.to(self.prev_span()));
            while self.at(Tok::LParen) {
                let (more, _) = self.parse_args();
                instance = self.ast.add_expr(Expr::Apply(instance, more), start.to(self.prev_span()));
            }
            self.ast.has_nowarn |= name == crate::names::NOWARN;
            self.main_annotated |= name == crate::names::MAIN;
            annots.push(Annot { name, args, instance });
            self.eat(Tok::Newline);
        }
        annots
    }

    /// Annotations of a type or an expression (`T @unchecked`, `e: @nowarn`). A line break after
    /// them ends the statement.
    pub(super) fn parse_inline_annots(&mut self) -> Vec<Annot> {
        let annots = self.parse_annots();
        if !annots.is_empty() && self.toks[self.pos - 1].kind == Tok::Newline {
            self.pos -= 1;
        }
        annots
    }

    /// The `[pkg]` of `private[pkg]` after its modifier; `private[this]` is plain private, its
    /// qualifier reported by the version (`Parsers.accessQualifierOpt`, over the brackets).
    fn parse_access_qualifier(&mut self) -> crate::intern::Name {
        let private = self.pos > 0 && self.toks[self.pos - 1].kind == Tok::KwPrivate;
        if !self.at(Tok::LBracket) {
            return names::EMPTY;
        }
        let open = self.bump().span;
        let this = self.eat(Tok::KwThis);
        let within = if this { names::EMPTY } else { self.expect_ident().0 };
        self.expect(Tok::RBracket);
        if this {
            let span = open.to(self.prev_span());
            self.migration_with(crate::source_version::Migration::RemoveThisQualifier, span, crate::warnings::NO_ID, || {
                let p = if private { "private" } else { "protected" };
                format!(
                    "Ignoring [this] qualifier.\nThe syntax `{p}[this]` will be deprecated in the future; just write `{p}` instead.\nSee: https://docs.scala-lang.org/scala3/reference/dropped-features/this-qualifier.html{}",
                    crate::source_version::rewrite_notice("This construct", "3.4-migration")
                )
            });
        }
        within
    }

    pub(super) fn parse_def(&mut self, out: &mut Vec<DefId>) {
        let (start, first) = (self.span().start, self.ast.defs.len());
        let annots = self.parse_annots();
        self.parse_annotated_def(annots, out);
        self.close_def_ranges(first, start);
    }

    /// The modifiers before a definition, where each was written, and the scope of a qualified
    /// `private[p]` or `protected[p]`.
    fn parse_modifier_run(&mut self) -> (Mods, [Span; MODIFIER_BITS], Name) {
        let mut m: Mods = 0;
        let mut spans = [Span::default(); MODIFIER_BITS];
        let mut within = names::EMPTY;
        loop {
            let bit = match self.kind() {
                Tok::KwPrivate => mods::PRIVATE,
                Tok::KwProtected => mods::PROTECTED,
                Tok::KwSealed => mods::SEALED,
                Tok::KwAbstract => mods::ABSTRACT,
                Tok::KwFinal => mods::FINAL,
                Tok::KwLazy => mods::LAZY,
                Tok::KwOverride => mods::OVERRIDE,
                Tok::KwImplicit => mods::IMPLICIT,
                Tok::KwCase if matches!(self.kind_at(1), Tok::KwClass | Tok::KwObject) => mods::CASE,
                _ => match self.soft_modifier_at(0) {
                    Some(bit) => {
                        self.reject_macro_modifier();
                        bit
                    }
                    None => break,
                },
            };
            if bit != 0 {
                if m & bit != 0 {
                    self.error_at(self.span(), format!("repeated modifier {}", modifier_word(bit)));
                }
                spans[bit.trailing_zeros() as usize] = self.span();
            }
            m |= bit;
            let was_access = matches!(self.kind(), Tok::KwPrivate | Tok::KwProtected);
            self.bump();
            if was_access {
                within = self.parse_access_qualifier();
            }
        }
        (m, spans, within)
    }

    fn reject_modifiers(&mut self, m: Mods, spans: &[Span; MODIFIER_BITS], allowed: Mods) {
        let mut rejected = m & !allowed;
        while rejected != 0 {
            let bit = rejected & rejected.wrapping_neg();
            rejected &= !bit;
            let span = spans[bit.trailing_zeros() as usize];
            if bit == mods::IMPLICIT && matches!(self.kind(), Tok::KwTrait | Tok::KwType) {
                self.error_at(span, "implicit modifier cannot be used for types or traits");
            } else {
                self.error_at(span, format!("modifier {} is not allowed for this definition", modifier_word(bit)));
            }
        }
    }

    fn parse_annotated_def(&mut self, annots: Vec<Annot>, out: &mut Vec<DefId>) {
        let first = out.len();
        let (m, spans, within) = self.parse_modifier_run();
        let access = mods::PRIVATE | mods::PROTECTED;
        let allowed = match self.kind() {
            Tok::KwVal if m & mods::LAZY != 0 => access | mods::FINAL | mods::LAZY | mods::OVERRIDE | mods::IMPLICIT,
            Tok::KwVal => access | mods::FINAL | mods::LAZY | mods::OVERRIDE | mods::IMPLICIT | mods::INLINE,
            Tok::KwVar => access | mods::FINAL | mods::OVERRIDE | mods::IMPLICIT,
            // `abstract override` marks a stackable modification, so `abstract` passes next to
            // `override`.
            Tok::KwDef if m & mods::OVERRIDE != 0 => access | mods::FINAL | mods::OVERRIDE | mods::INFIX | mods::INLINE | mods::TRANSPARENT | mods::ABSTRACT | mods::IMPLICIT,
            Tok::KwDef => access | mods::FINAL | mods::OVERRIDE | mods::INFIX | mods::INLINE | mods::TRANSPARENT | mods::IMPLICIT,
            Tok::KwClass => !(mods::LAZY | mods::OPAQUE | mods::INLINE),
            Tok::KwTrait => !(mods::FINAL | mods::LAZY | mods::OPAQUE | mods::IMPLICIT | mods::INLINE),
            Tok::KwObject => access | mods::FINAL | mods::CASE | mods::OVERRIDE | mods::LAZY | mods::IMPLICIT | mods::INLINE,
            Tok::KwEnum => access | mods::INFIX,
            Tok::KwType => access | mods::OPAQUE | mods::FINAL | mods::OVERRIDE | mods::INFIX,
            Tok::KwGiven => access | mods::FINAL | mods::OVERRIDE | mods::LAZY | mods::INLINE | mods::TRANSPARENT,
            Tok::Ident if self.tok().name == names::EXTENSION => 0,
            _ => !0,
        };
        self.reject_modifiers(m, &spans, allowed);
        let m = if annots.iter().any(|a| a.name == names::DEPRECATED) {
            if !self.syntax.std {
                crate::typer::deprecation::note_deprecated();
            }
            m | mods::DEPRECATED
        } else {
            m
        };
        match self.kind() {
            Tok::KwVal | Tok::KwVar => self.parse_val(m, annots, out),
            Tok::KwDef => {
                let d = self.parse_fun(m, annots, &[], &[]);
                self.push_named(out, d);
            }
            Tok::KwClass | Tok::KwTrait | Tok::KwObject | Tok::KwEnum => {
                let is_class = self.at(Tok::KwClass);
                let d = self.parse_class(m, annots);
                if self.push_named(out, d) && is_class && m & mods::IMPLICIT != 0 {
                    if let Some(conversion) = self.implicit_class_conversion(d) {
                        out.push(conversion);
                    }
                }
            }
            Tok::KwType => {
                let d = self.parse_type_def(m, annots);
                self.push_named(out, d);
            }
            Tok::KwGiven => {
                let d = self.parse_given(m, annots);
                self.push_named(out, d);
            }
            Tok::Ident if self.tok().name == names::EXTENSION => self.parse_extension(m, out),
            _ => {
                let (at, found) = self.found();
                self.error_at(at, format!("expected a definition, found {}", found));
            }
        }
        if within != names::EMPTY {
            for d in &out[first..] {
                let at = self.ast.def(*d).span.start;
                self.ast.access_scopes.push((at, within));
            }
        }
    }

    /// A definition whose name parsed joins `out`; one whose name did not is dropped with all it
    /// holds.
    #[inline]
    fn push_named(&mut self, out: &mut Vec<DefId>, d: DefId) -> bool {
        let named = self.ast.def(d).name != names::EMPTY;
        if named {
            out.push(d);
        }
        named
    }

    /// The end of a definition's header after its name, `mark` the error count there and
    /// `typed` whether a result type parsed: whether the header is complete, and whether a body
    /// follows without its `=`. The header is incomplete when it reported an error, or when it
    /// stops at a token that neither continues it nor ends the statement, which is reported as
    /// the `=` missing: the tokens up to a `=` on the header's line are skipped; without one an
    /// expression after the result type is the body (`def f(x: Int): Int x + 1`), and without a
    /// result type what is left of the line is skipped with the header (`def f(x: Int) Int [`),
    /// as scalac's skip to the end of the statement does.
    #[inline]
    fn header_complete(&mut self, mark: u32, typed: bool) -> (bool, bool) {
        if self.at_header_end() {
            return (self.error_events == mark, false);
        }
        self.header_breaks(typed)
    }

    #[cold]
    #[inline(never)]
    fn header_breaks(&mut self, typed: bool) -> (bool, bool) {
        let (at, found) = self.found();
        self.error_at(at, format!("expected '=', found {}", found));
        if self.skip_header_to(|p, i| p.toks[i].kind == Tok::Eq) {
            return (false, false);
        }
        if typed {
            return (false, self.expr_start_at(0) && !self.at(Tok::Indent));
        }
        self.skip_header_line();
        (false, false)
    }

    /// `val a, b = e` defines each name with its own copy of the type and initialiser, read again
    /// from the tokens.
    fn parse_val(&mut self, mut m: Mods, annots: Vec<Annot>, out: &mut Vec<DefId>) {
        if self.bump().kind == Tok::KwVar {
            m |= mods::MUTABLE;
        }
        // A name followed by a token no pattern continues with is the val's name: `val m Meter(4)`
        // lacks its `=`.
        let simple = (self.at(Tok::Ident)
            && !matches!(
                self.kind_at(1),
                Tok::LParen | Tok::LBracket | Tok::Dot | Tok::At | Tok::OpIdent | Tok::Arrow | Tok::LArrow | Tok::Underscore
            ))
            // An operator's name is the val's too, as dotty's `simplePattern` takes any identifier:
            // tyrian's `val ==: = Batch.==:`.
            || (self.at(Tok::OpIdent) && (matches!(self.kind_at(1), Tok::Eq | Tok::Comma) || self.written_colon_at(1)));
        // `val _: T = e` runs and checks `e` under a name nothing can use.
        let unnamed = self.at(Tok::Underscore) && (self.written_colon_at(1) || self.kind_at(1) == Tok::Eq);
        let (name, span, pat) = if simple {
            let (n, s) = self.expect_ident();
            self.check_dollar_name(n, s);
            (n, s, None)
        } else if unnamed {
            let s = self.bump().span;
            (self.fresh_name("unnamed"), s, None)
        } else {
            let s = self.span();
            let p = self.parse_pattern();
            self.reject_given_patterns(p);
            (names::EMPTY, s.to(self.prev_span()), Some(p))
        };
        let mut more_names = Vec::new();
        while simple && self.eat(Tok::Comma) {
            let (n, s) = self.expect_ident();
            self.check_dollar_name(n, s);
            more_names.push((n, s));
        }
        let rest_at = self.pos;
        let group_start = out.len();
        let mark = self.error_events;
        let mut define = |p: &mut Self, name: Name, span: Span, pat: Option<PatId>| {
            let colon = p.eat_annotation_colon();
            let region = colon.flatten();
            let ty = colon.map(|_| p.parse_type());
            p.annotation_before_eq(region);
            let (complete, bare_body) = p.header_complete(mark, ty.is_some());
            let m = if complete { m } else { m | mods::INCOMPLETE };
            let rhs = if bare_body {
                Some(p.parse_expr())
            } else if !p.at(Tok::Eq) {
                None
            } else if m & mods::MUTABLE != 0 && ty.is_some() && p.default_init_at(1) {
                p.bump();
                let span = p.bump().span;
                let msg = format!(
                    "`= _` has been deprecated; use `= uninitialized` instead.\n`uninitialized` can be imported with `scala.compiletime.uninitialized`.{}",
                    crate::source_version::rewrite_notice("This construct", "3.4-migration")
                );
                p.migration(crate::source_version::Migration::UninitializedVars, span, msg, crate::warnings::NO_ID);
                let init = p.uninitialized_path(span);
                p.ast.default_inits.push(init);
                Some(init)
            } else {
                p.bump();
                Some(p.parse_rhs(region))
            };
            if rhs.is_none() && ty.is_none() && complete {
                p.error_at(span, "a val needs a type or an initializer");
            }
            p.end_annotation_region(region);
            let kind = DefKind::Val { pat, ty, rhs };
            let d = p.ast.add_def(Def { name, span, mods: m, annots: annots.clone(), kind });
            if pat.is_some() {
                p.pattern_vals += 1;
            }
            out.push(d);
        };
        define(self, name, span, pat);
        let errors_reported = self.errors.len();
        for (name, span) in more_names {
            self.pos = rest_at;
            define(self, name, span, None);
            self.errors.truncate(errors_reported);
        }
        // The copies of an initialiser, typed in this order (`Ast::val_copies`); names without
        // one have nothing to order.
        if matches!(self.ast.def(out[group_start]).kind, DefKind::Val { rhs: Some(_), .. }) {
            for i in group_start + 1..out.len() {
                self.ast.val_copies.insert(out[i], out[i - 1]);
            }
        }
    }

    /// `var x: T = _`: the variable starts at the zero of its type.
    fn default_init_at(&self, n: usize) -> bool {
        self.kind_at(n) == Tok::Underscore && matches!(self.kind_at(n + 1), Tok::Newline | Tok::Outdent | Tok::Eof | Tok::Semi | Tok::RBrace)
    }

    /// `_root_.scala.compiletime.uninitialized`, what scalac reads `var x: T = _` as.
    fn uninitialized_path(&mut self, span: Span) -> ExprId {
        let mut e = self.ast.add_expr(Expr::Ident(names::ROOT), span);
        for seg in ["scala", "compiletime", "uninitialized"] {
            let n = self.interner.intern(seg);
            e = self.ast.add_expr(Expr::Select(e, n), span);
        }
        e
    }

    /// A parameter clause may start on the line after the previous one; a blank line in between
    /// ends the signature, as in scalac.
    fn param_clause_on_next_line(&self, n: usize) -> bool {
        if self.kind_at(n) != Tok::Newline || self.kind_at(n + 1) != Tok::LParen {
            return false;
        }
        let from = self.toks[(self.pos + n).saturating_sub(1)].span.end as usize;
        let to = self.toks[self.pos + n + 1].span.start as usize;
        let mut line_is_blank = false;
        for &b in &self.text.as_bytes()[from..to] {
            if b == b'\n' {
                if line_is_blank {
                    return false;
                }
                line_is_blank = true;
            } else if b > b' ' {
                line_is_blank = false;
            }
        }
        true
    }

    /// The parameter clauses here, after `lead` term parameters of clauses before them (an
    /// extension's receiver clauses for its methods), which an anonymous using parameter's name
    /// counts as scalac's parser does (`typesToParams`): `x$N`, N its place among all of them.
    pub(super) fn parse_param_clauses(&mut self, class_params: Option<bool>, lead: usize) -> Vec<ParamClause> {
        let mut clauses = Vec::new();
        let mut lead = lead;
        loop {
            if self.param_clause_on_next_line(0) {
                self.bump();
            }
            if !self.at(Tok::LParen) {
                break;
            }
            self.bump();
            let mut clause = ParamClause::default();
            let mut cut = false;
            if self.at_soft(names::USING) && self.kind_at(1) != Tok::Colon {
                self.bump();
                clause.is_using = true;
            } else if self.at(Tok::KwImplicit) {
                // Reported by the version (`Parsers.paramMods`).
                let at = self.bump().span;
                self.migration_with(Migration::Scala2Implicits, at, crate::warnings::NO_ID, || {
                    format!("`implicit` parameters are no longer supported, use a `using` clause instead{}", crate::source_version::rewrite_notice("This construct", "future-migration"))
                });
                clause.is_using = true;
                clause.is_implicit = true;
            }
            while !self.at(Tok::RParen) && !self.at(Tok::Eof) {
                let annots = if self.at(Tok::At) {
                    let annots = self.parse_annots();
                    push_list(&mut self.ast.param_annots, &annots)
                } else {
                    ListRef::EMPTY
                };
                let mut pm: Mods = 0;
                let mut within = names::EMPTY;
                loop {
                    match self.kind() {
                        Tok::KwPrivate => {
                            pm |= mods::PRIVATE;
                            self.bump();
                            within = self.parse_access_qualifier();
                            continue;
                        }
                        Tok::KwProtected => {
                            pm |= mods::PROTECTED;
                            self.bump();
                            within = self.parse_access_qualifier();
                            continue;
                        }
                        Tok::KwOverride => pm |= mods::OVERRIDE,
                        Tok::KwFinal => pm |= mods::FINAL,
                        // A class parameter's own `implicit` (`class C(x: A, implicit val y: B)`)
                        // makes its accessor an implicit member, as dotty's `param` reads any
                        // modifier of a class parameter (`modifiers()`); a clause's leading one
                        // marks the clause.
                        Tok::KwImplicit if class_params.is_some() => pm |= mods::IMPLICIT,
                        Tok::KwVal => pm |= mods::FIELD,
                        Tok::KwVar => pm |= mods::FIELD | mods::MUTABLE,
                        Tok::Ident if self.tok().name == names::INLINE && self.kind_at(1) == Tok::Ident => pm |= mods::INLINE,
                        Tok::Ident if self.tok().name == names::ERASED && self.kind_at(1) == Tok::Ident => {
                            self.reject_macro_modifier();
                        }
                        _ => break,
                    }
                    self.bump();
                }
                // dotty's `param` of a class: a modifier other than `inline` or `erased` asks for `val` or `var`, which
                // alone set FIELD so far.
                let modifiers = mods::PRIVATE | mods::PROTECTED | mods::OVERRIDE | mods::FINAL | mods::IMPLICIT;
                if pm & modifiers != 0 && pm & mods::FIELD == 0 && class_params.is_some() {
                    self.error_at(self.span(), "`val` or `var` expected");
                }
                if class_params == Some(true) && !clause.is_using && clauses.is_empty() {
                    pm |= mods::FIELD;
                }
                let anonymous_using = clause.is_using
                    && !(self.at(Tok::Ident) && self.written_colon_at(1));
                let mut region = None;
                let (name, span) = if anonymous_using {
                    let n = lead + clause.params.len() + 1;
                    (self.interner.intern(&format!("x${}", n)), self.span())
                } else {
                    let ident = self.expect_ident();
                    self.check_dollar_name(ident.0, ident.1);
                    match self.eat_annotation_colon() {
                        Some(r) => region = r,
                        None => {
                            self.expect(Tok::Colon);
                        }
                    }
                    ident
                };
                let ty_start = self.span();
                let ty = self.parse_param_type();
                let ty_span = ty_start.to(self.prev_span());
                if pm & mods::FIELD != 0 && matches!(self.ast.ty(ty), TyExpr::ByName(_)) {
                    self.error_at(ty_span, "val parameters may not be call-by-name");
                }
                self.annotation_before_eq(region);
                let default = if self.eat(Tok::Eq) { Some(self.parse_default(region)) } else { None };
                self.end_annotation_region(region);
                if within != names::EMPTY {
                    self.ast.access_scopes.push((span.start, within));
                }
                if name != names::EMPTY {
                    clause.params.push(Param { name, span, ty, default, mods: pm, annots });
                }
                if !self.list_continues(Tok::RParen, &mut cut) {
                    break;
                }
                if matches!(self.ast.ty(ty), TyExpr::Repeated(_)) && !self.at(Tok::RParen) {
                    self.error_at(ty_span, "a vararg parameter must come last");
                }
            }
            self.expect(Tok::RParen);
            let last = clause.is_implicit;
            lead += clause.params.len();
            clauses.push(clause);
            // An implicit clause is the last one, as in scalac.
            if last {
                break;
            }
        }
        clauses
    }

    /// `implicit class C[T](x: A)(implicit e: E)` also defines the conversion
    /// `implicit def C[T](x: A)(implicit e: E): C[T] = new C[T](x)`, as scalac desugars it.
    fn implicit_class_conversion(&mut self, class_def: DefId) -> Option<DefId> {
        let def = self.ast.def(class_def);
        let DefKind::Class(cls) = &def.kind else { return None };
        let (name, span, class_mods) = (def.name, def.span, def.mods);
        if class_mods & mods::CASE != 0 {
            self.error_at(span, "A case class may not be defined as implicit");
            return None;
        }
        let plain: Vec<&ParamClause> = cls.clauses.iter().filter(|c| !c.is_using).collect();
        if plain.len() != 1 || plain[0].params.len() != 1 {
            self.error_at(span, "Implicit classes must accept exactly one primary constructor parameter");
            return None;
        }
        let tparams: Vec<TypeParam> = cls
            .tparams
            .iter()
            .map(|tp| TypeParam { variance: 0, annots: Vec::new(), ..tp.clone() })
            .collect();
        let clauses: Vec<ParamClause> = cls
            .clauses
            .iter()
            .map(|c| ParamClause {
                params: c.params.iter().map(|p| Param { mods: 0, annots: ListRef::EMPTY, ..*p }).collect(),
                is_using: c.is_using,
                is_implicit: c.is_implicit,
            })
            .collect();
        let mut ty = self.ast.add_ty(TyExpr::Name(name), span);
        if !tparams.is_empty() {
            let mark = self.ty_scratch.len();
            for tp in &tparams {
                let arg = self.ast.add_ty(TyExpr::Name(tp.name), span);
                self.ty_scratch.push(arg);
            }
            let args = self.ty_list(mark);
            ty = self.ast.add_ty(TyExpr::Apply(ty, args), span);
        }
        let mark = self.expr_scratch.len();
        for p in &clauses[0].params {
            let arg = self.ast.add_expr(Expr::Ident(p.name), p.span);
            self.expr_scratch.push(arg);
        }
        let args = self.expr_list(mark);
        let body = Some(self.ast.add_expr(Expr::New(ty, args), span));
        let fun = FunDef {
            tparams,
            clauses,
            ret: Some(ty),
            body,
            ext_tparams: 0,
            ext_clauses: 0,
            is_extension: false,
            ext_group: 0,
        };
        let m = class_mods & (mods::IMPLICIT | mods::PRIVATE | mods::PROTECTED | mods::INCOMPLETE);
        Some(self.ast.add_def(Def { name, span, mods: m, annots: Vec::new(), kind: DefKind::Fun(Box::new(fun)) }))
    }

    fn parse_fun(
        &mut self,
        m: Mods,
        annots: Vec<Annot>,
        ext_tparams: &[TypeParam],
        ext_clauses: &[ParamClause],
    ) -> DefId {
        self.expect(Tok::KwDef);
        if self.at(Tok::KwThis) && ext_clauses.is_empty() {
            return self.parse_secondary_ctor(m, annots);
        }
        let (name, span) = self.expect_ident();
        self.check_dollar_name(name, span);
        let mark = self.error_events;
        let mut tparams = ext_tparams.to_vec();
        tparams.extend(self.parse_type_params());
        let mut clauses = ext_clauses.to_vec();
        let lead = ext_clauses.iter().map(|c| c.params.len()).sum();
        clauses.extend(self.parse_param_clauses(None, lead));
        let colon = self.eat_annotation_colon();
        let region = colon.flatten();
        let ret = colon.map(|_| self.parse_type());
        self.annotation_before_eq(region);
        let (complete, bare_body) = self.header_complete(mark, ret.is_some());
        let m = if complete { m } else { m | mods::INCOMPLETE };
        let body = if self.eat(Tok::Eq) || bare_body { Some(self.parse_rhs(region)) } else { None };
        self.end_annotation_region(region);
        let fun = FunDef {
            tparams,
            clauses,
            ret,
            body,
            ext_tparams: ext_tparams.len() as u8,
            ext_clauses: ext_clauses.len() as u8,
            is_extension: !ext_clauses.is_empty(),
            ext_group: if ext_clauses.is_empty() { 0 } else { self.ext_groups },
        };
        self.ast.add_def(Def { name, span, mods: m, annots, kind: DefKind::Fun(Box::new(fun)) })
    }

    /// `def this(params) = this(args)` or `= { this(args); stmts }`: a def named `<init>` whose
    /// body starts with the self constructor call.
    fn parse_secondary_ctor(&mut self, m: Mods, annots: Vec<Annot>) -> DefId {
        let span = self.bump().span;
        let mark = self.error_events;
        let clauses = self.parse_param_clauses(None, 0);
        if self.at(Tok::Colon) {
            self.error_at(self.span(), "'=' expected, but ':' found");
        }
        // Its body follows whatever the header did: it has no result type to wait for.
        let m = if self.header_complete(mark, true).0 { m } else { m | mods::INCOMPLETE };
        self.expect(Tok::Eq);
        let body = self.parse_block_or_expr();
        if let Some(at) = self.first_statement_not_self_call(body) {
            self.error_at(self.ast.expr_span(at), "'this' expected, but another expression found");
        }
        let fun = FunDef {
            tparams: Vec::new(),
            clauses,
            ret: None,
            body: Some(body),
            ext_tparams: 0,
            ext_clauses: 0,
            is_extension: false,
            ext_group: 0,
        };
        self.ast.add_def(Def { name: names::INIT, span, mods: m, annots, kind: DefKind::Fun(Box::new(fun)) })
    }

    /// The first statement of a secondary constructor's body when it is no `this(...)` call.
    fn first_statement_not_self_call(&self, body: ExprId) -> Option<ExprId> {
        let first = match self.ast.expr(body) {
            Expr::Block(stmts) => match self.ast.stmt_list(stmts).first() {
                Some(Stmt::Expr(e)) => *e,
                _ => return Some(body),
            },
            _ => body,
        };
        let mut head = first;
        loop {
            match self.ast.expr(head) {
                Expr::Apply(f, _) | Expr::UsingApply(f, _) => head = f,
                Expr::This if head != first => return None,
                _ => return Some(first),
            }
        }
    }

    /// `type T = X`, `type T[X] <: U`, `type T >: L <: U`, and an opaque alias with its bound.
    pub(super) fn parse_type_def(&mut self, m: Mods, annots: Vec<Annot>) -> DefId {
        self.expect(Tok::KwType);
        let (name, span) = self.expect_ident();
        self.check_dollar_name(name, span);
        let mark = self.error_events;
        let tparams = self.parse_type_params();
        if let Some(tp) = tparams.iter().find(|tp| !tp.context_bounds.is_empty()) {
            self.error_at(tp.span, "Context bounds are not allowed in this position");
        }
        let lower = self.eat(Tok::Supertype).then(|| self.parse_type());
        let upper = self.eat(Tok::Subtype).then(|| self.parse_type());
        let rhs = self.eat(Tok::Eq).then(|| self.parse_type());
        if let (Some(r), true, false) = (rhs, tparams.is_empty(), self.ast.notes.binder_annots.is_empty()) {
            self.own_alias_clause(r);
        }
        if rhs.is_none() && m & mods::OPAQUE != 0 {
            self.error_at(span, "an opaque type needs a right-hand side");
        }
        let m = if self.error_events == mark { m } else { m | mods::INCOMPLETE };
        let kind = DefKind::TypeAlias { tparams, rhs, lower, upper };
        self.ast.add_def(Def { name, span, mods: m, annots, kind })
    }

    /// A parameterless alias whose right side is a lambda written there, not in parentheses, is the lambda's binders'
    /// owner, whose completer has them (`Namer.TypeDefCompleter.completerTypeParams` of `TypeDef(_,
    /// LambdaTypeTree(..))`, a `Parens` no such tree): its clause is the alias's own.
    #[cold]
    #[inline(never)]
    fn own_alias_clause(&mut self, rhs: TyExprId) {
        let start = self.ast.ty_spans[rhs.idx()].start;
        let at = self.toks.partition_point(|t| t.span.start < start);
        let before = self.toks[..at].iter().rev().find(|t| !matches!(t.kind, Tok::Newline | Tok::Indent | Tok::Outdent));
        if !before.is_some_and(|t| t.kind == Tok::Eq) {
            return;
        }
        for a in self.ast.notes.binder_annots.iter_mut().filter(|a| a.clause == BinderClause::Lambda(rhs)) {
            a.clause = BinderClause::Own(rhs);
        }
        for n in self.ast.notes.binder_names.iter_mut().filter(|n| n.0 == BinderClause::Lambda(rhs)) {
            n.0 = BinderClause::Own(rhs);
        }
    }

    /// An extension's methods, a language import among them scoped to them (dotty's
    /// `extMethods` restoring `languageImportContext`).
    fn parse_extension(&mut self, m: Mods, out: &mut Vec<DefId>) {
        let saved = self.syntax.postfix_ops;
        self.parse_extension_scoped(m, out);
        self.syntax.postfix_ops = saved;
    }

    fn parse_extension_scoped(&mut self, m: Mods, out: &mut Vec<DefId>) {
        self.bump();
        self.ext_groups += 1;
        let mark = self.error_events;
        let tparams = self.parse_type_params();
        let clauses = self.parse_param_clauses(None, 0);
        if clauses.iter().filter(|c| !c.is_using).count() != 1 {
            self.error_at(self.prev_span(), "an extension needs exactly one receiver parameter");
        }
        // The receiver is part of each method's header.
        let m = if self.error_events == mark { m } else { m | mods::INCOMPLETE };
        // The group's methods, which an export clause among them names its companion extension
        // method among (`ExtensionExport::methods`).
        let headers_mark = self.ast.extension_exports.len();
        let mut methods: Vec<DefId> = Vec::new();
        // An export clause as the group's one member (`Parsers.extension`: `if in.token == EXPORT
        // then exportClause()`).
        if self.at(Tok::KwExport) {
            self.parse_extension_export(&tparams, &clauses);
            return;
        }
        if matches!(self.kind(), Tok::At | Tok::KwDef) || is_hard_modifier(self.kind()) || self.soft_modifier_at(0).is_some() {
            let annots = self.parse_annots();
            let (fm, within) = self.parse_ext_method_mods(m);
            if self.at(Tok::KwDef) {
                let d = self.parse_fun(fm, annots, &tparams, &clauses);
                self.note_access_scope(d, within);
                self.push_named(out, d);
            } else {
                self.error_at(self.span(), "expected extension methods");
            }
            return;
        }
        if self.at(Tok::ColonEol) && self.kind_at(1) == Tok::Indent {
            self.bump();
        }
        if !self.eat(Tok::Indent) {
            self.error_at(self.span(), "expected extension methods");
            return;
        }
        self.statements(RecoverySite::Extension, |p| {
            let (start, first) = (p.span().start, p.ast.defs.len());
            // `ExtMethods ::= ExtMethod | ...`, `ExtMethod ::= ... 'def' DefDef | Export`.
            if p.at(Tok::KwExport) {
                p.parse_extension_export(&tparams, &clauses);
                return;
            }
            let annots = p.parse_annots();
            let (fm, within) = p.parse_ext_method_mods(m);
            if p.at(Tok::KwDef) {
                let d = p.parse_fun(fm, annots, &tparams, &clauses);
                p.note_access_scope(d, within);
                p.push_named(out, d);
                p.close_def_ranges(first, start);
                methods.push(d);
            } else {
                p.error_at(p.span(), "only defs are allowed in an extension");
            }
        });
        self.end_region();
        for h in &mut self.ast.extension_exports[headers_mark..] {
            h.methods = methods.clone();
        }
    }

    /// An export clause inside an extension group: parsed as any export clause of the enclosing
    /// body, its clauses noted with the group's header (`Ast::extension_exports`).
    fn parse_extension_export(&mut self, tparams: &[TypeParam], clauses: &[ParamClause]) {
        let mark = self.export_scratch.len();
        self.parse_export();
        let mut starts: Vec<u32> = self.export_scratch[mark..].iter().map(|imp| imp.span.start).collect();
        starts.dedup();
        self.ast.extension_exports.push(ExtensionExport { starts, tparams: tparams.to_vec(), clauses: clauses.to_vec(), methods: Vec::new() });
    }

    /// The modifiers of an extension method, after the extension's own `m`.
    /// The modifiers of an extension method, after the extension's own `m`, as a `def` takes
    /// them, with the scope of a qualified access modifier.
    fn parse_ext_method_mods(&mut self, m: Mods) -> (Mods, Name) {
        let (fm, spans, within) = self.parse_modifier_run();
        let allowed = mods::PRIVATE | mods::PROTECTED | mods::FINAL | mods::OVERRIDE | mods::INFIX | mods::INLINE | mods::TRANSPARENT | mods::IMPLICIT;
        let allowed = if fm & mods::OVERRIDE != 0 { allowed | mods::ABSTRACT } else { allowed };
        if self.at(Tok::KwDef) {
            self.reject_modifiers(fm, &spans, allowed);
        }
        (m | fm, within)
    }

    fn note_access_scope(&mut self, d: DefId, within: Name) {
        if within != names::EMPTY {
            let at = self.ast.def(d).span.start;
            self.ast.access_scopes.push((at, within));
        }
    }

    pub(super) fn parse_parents(&mut self) -> Vec<Parent> {
        let mut parents = Vec::new();
        if !self.eat(Tok::KwExtends) {
            return parents;
        }
        loop {
            self.template_type = true;
            let ty = self.parse_simple_type();
            self.template_type = false;
            // `extends [A, B]`, the parent's name missing: its arguments go with it.
            if matches!(self.ast.ty(ty), TyExpr::Error) && self.at(Tok::LBracket) {
                self.parse_type_args();
            }
            let mut args = Vec::new();
            let mut ends = Vec::new();
            while self.at(Tok::LParen) {
                args.push(self.parse_args());
                ends.push(self.prev_span().end);
            }
            parents.push(Parent { ty, args, ends });
            if !self.eat(Tok::Comma) && !self.eat(Tok::KwWith) {
                break;
            }
        }
        parents
    }

    /// The type classes of a `derives` clause, each with the last name of its path.
    fn parse_derives(&mut self) -> Vec<(TyExprId, Name)> {
        let mut derives = Vec::new();
        // dotc's `newLinesOptWhenFollowedBy(nme.derives)`: the clause on the line after the header.
        if self.at(Tok::Newline) && self.kind_at(1) == Tok::Ident && self.toks.get(self.pos + 1).is_some_and(|t| t.name == names::DERIVES) {
            self.bump();
        }
        if self.at_soft(names::DERIVES) {
            self.bump();
            loop {
                derives.push(self.parse_class_path());
                if !self.eat(Tok::Comma) {
                    break;
                }
            }
        }
        derives
    }

    fn parse_class(&mut self, m: Mods, annots: Vec<Annot>) -> DefId {
        let kind = match self.bump().kind {
            Tok::KwClass => ClassKind::Class,
            Tok::KwTrait => ClassKind::Trait,
            Tok::KwObject => ClassKind::Object,
            _ => ClassKind::Enum,
        };
        let (name, span) = self.expect_ident();
        self.check_dollar_name(name, span);
        let mark = self.error_events;
        let tparams = self.parse_type_params();
        let mut m = m;
        if matches!(self.kind(), Tok::KwPrivate | Tok::KwProtected)
            && matches!(self.kind_at(1), Tok::LParen | Tok::LBracket)
        {
            let is_private = self.bump().kind == Tok::KwPrivate;
            let within = self.parse_access_qualifier();
            m |= if is_private { mods::PRIVATE_CTOR } else { mods::PROTECTED_CTOR };
            if within != names::EMPTY {
                // Filed under the end of the class name, where no definition starts.
                self.ast.access_scopes.push((span.end, within));
            }
        }
        let is_case = m & mods::CASE != 0;
        // An object has no parameters, so a `(` on the next line starts a statement of its own.
        let clauses = if kind == ClassKind::Object && !self.at(Tok::LParen) {
            Vec::new()
        } else {
            self.parse_param_clauses(Some(is_case), 0)
        };
        let parents = self.parse_parents();
        let derives = self.parse_derives();
        // A `:` that ends the header's line with no indented body after it, where no line is indented
        // further or indentation is not significant: dotty's `acceptIndent` past the colon (an end marker
        // right after it ends an empty body, which the lexer gives).
        let colon_alone = self.at(Tok::ColonEol) && self.kind_at(1) != Tok::Indent && self.text.as_bytes()[self.span().start as usize] == b':';
        if colon_alone {
            self.bump();
            let t = self.tok();
            let what = match t.kind {
                Tok::Ident => format!("identifier {}", self.interner.get(t.name)),
                k => k.describe().trim_matches('\'').to_string(),
            };
            self.error_at(t.span, format!("indented definitions expected, {} found", what));
        } else if !(self.at_header_end() || self.at_template_body()) {
            // `class C(x: Int) extend B:`: what stands before the body on the header's line is
            // skipped to it.
            let (at, found) = self.found();
            self.error_at(at, format!("expected 'extends' or a template body, found {}", found));
            self.skip_header_to(|p, i| {
                let k = p.toks[i].kind;
                (k == Tok::ColonEol && p.toks.get(i + 1).is_some_and(|t| t.kind == Tok::Indent))
                    || (k == Tok::Indent && p.toks[i].span.end > p.toks[i].span.start)
            });
        }
        if self.error_events != mark {
            m |= mods::INCOMPLETE;
        }
        let (body, exports, self_type, self_alias) = self.parse_template_body(kind == ClassKind::Enum);
        let has_case = |p: &Self| {
            body.iter().any(|s| {
                matches!(s, Stmt::Def(d) if matches!(&p.ast.def(*d).kind, DefKind::Class(c) if c.kind == ClassKind::EnumCase))
            })
        };
        if kind == ClassKind::Enum && !has_case(self) {
            let msg = format!("enum {} needs at least one case", self.interner.get(name));
            self.error_at(span, msg);
        }
        if !derives.is_empty() {
            let is_object = kind == ClassKind::Object;
            self.pending_derives.push(PendingDerives { class: name, span, tparams: tparams.clone(), derives, is_object });
        }
        if kind == ClassKind::Class && m & mods::CASE != 0 {
            self.pending_cases.push((name, span));
        } else if matches!(kind, ClassKind::Class | ClassKind::Trait) && self.constructor_has_default(&clauses, &body) {
            self.pending_defaults.push((name, span));
        }
        let class = ClassDef { kind, tparams, clauses, parents, body, exports, self_type, self_alias };
        self.ast.add_def(Def { name, span, mods: m, annots, kind: DefKind::Class(Box::new(class)) })
    }

    /// The rest of `new T(args) with U { ... }` after the first parent and its arguments: the
    /// definition of the anonymous class, whose name the typer makes up.
    pub(super) fn parse_anon_class(&mut self, first: TyExprId, args: Vec<(ListRef, bool)>, ends: Vec<u32>, start: Span) -> ExprId {
        let mut parents = vec![Parent { ty: first, args, ends }];
        while self.eat(Tok::KwWith) {
            self.template_type = true;
            let ty = self.parse_simple_type();
            self.template_type = false;
            let mut args = Vec::new();
            let mut ends = Vec::new();
            while self.at(Tok::LParen) {
                args.push(self.parse_args());
                ends.push(self.prev_span().end);
            }
            parents.push(Parent { ty, args, ends });
        }
        self.parse_anon_class_with(parents, start)
    }

    /// The template of an anonymous class with these parents, none for `new:` or `new { ... }`.
    pub(super) fn parse_anon_class_with(&mut self, parents: Vec<Parent>, start: Span) -> ExprId {
        let (body, exports, self_type, self_alias) = self.parse_template_body(false);
        let span = start.to(self.prev_span());
        let class = ClassDef { kind: ClassKind::Class, tparams: Vec::new(), clauses: Vec::new(), parents, body, exports, self_type, self_alias };
        let def = self.ast.add_def(Def {
            name: names::DOLLAR_ANON,
            span,
            mods: mods::FINAL,
            annots: Vec::new(),
            kind: DefKind::Class(Box::new(class)),
        });
        // The class stands where its `new` does, to the end of its template, as scalac's tree
        // places it; not the definition whose body holds it (`close_def_ranges`).
        self.ast.def_ranges[def.idx()] = span;
        self.ast.add_expr(Expr::NewAnon(def), span)
    }

    /// The companions `add_derived_givens` made, each placed right after its class among a body's
    /// `items`, as `Desugar.classDef` gives the class with its companion (`cdef1 :: companions`), in
    /// one pass.
    pub(super) fn place_companions<T>(&self, items: &mut Vec<T>, companions: Vec<DefId>, def_of: impl Fn(&T) -> Option<DefId>, wrap: impl Fn(DefId) -> T) {
        if companions.is_empty() {
            return;
        }
        let mut by_class: FxMap<Name, DefId> = companions.iter().map(|&o| (self.ast.def(o).name, o)).collect();
        let mut placed = Vec::with_capacity(items.len() + companions.len());
        for item in std::mem::take(items) {
            let companion = def_of(&item).and_then(|d| {
                let def = self.ast.def(d);
                let class = matches!(&def.kind, DefKind::Class(c) if c.kind != ClassKind::Object);
                if class { by_class.remove(&def.name) } else { None }
            });
            placed.push(item);
            if let Some(o) = companion {
                placed.push(wrap(o));
            }
        }
        // A companion whose class the body does not hold goes last, in its order.
        placed.extend(companions.into_iter().filter(|o| by_class.contains_key(&self.ast.def(*o).name)).map(wrap));
        *items = placed;
    }

    /// `derives TC` on `C[A]` stands for `given derived$TC[A: TC]: TC[C[A]] = TC.derived` in the
    /// companion of `C`, and a case class has a companion (`Desugar.classDef`'s `companionDefs`).
    /// `defs` are the definitions of the body that holds the classes recorded since `mark` and
    /// `cases_mark`; a companion that the body lacks is made, synthetic
    /// (`Ast::synthetic_companions`), and returned for the caller to add.
    pub(super) fn add_derived_givens(&mut self, mark: usize, cases_mark: usize, defaults_mark: usize, defs: &[DefId]) -> Vec<DefId> {
        let mut companions = Vec::new();
        let mut objects: FxMap<Name, DefId> = FxMap::default();
        // A method of the class's name beside it does not keep the companion away: the namer holds
        // the module and the methods as alternatives of one name, as scalac overloads them
        // (`def A(s: String)` beside `object A`), and a value of its name clashes with the
        // companion, as scalac reports.
        // The ranges of the body's classes, which their made companions take.
        let mut class_ranges: FxMap<Name, Span> = FxMap::default();
        for &d in defs {
            let def = self.ast.def(d);
            match &def.kind {
                DefKind::Class(c) if c.kind == ClassKind::Object => {
                    objects.insert(def.name, d);
                }
                DefKind::Class(_) => {
                    class_ranges.entry(def.name).or_insert(self.ast.def_range(d));
                }
                _ => {}
            }
        }
        for pending in self.pending_derives.split_off(mark) {
            let givens: Vec<Stmt> =
                pending.derives.iter().map(|&(tc, name)| Stmt::Def(self.derived_given(&pending, tc, name))).collect();
            match objects.get(&pending.class).copied() {
                Some(d) => {
                    if let DefKind::Class(c) = &mut self.ast.defs[d.idx()].kind {
                        c.body.extend(givens);
                    }
                }
                None => {
                    let range = class_ranges.get(&pending.class).copied().unwrap_or(pending.span);
                    let companion = self.synthetic_companion(pending.class, pending.span, range, givens);
                    objects.insert(pending.class, companion);
                    companions.push(companion);
                }
            }
        }
        let defaults = self.pending_defaults.split_off(defaults_mark.min(self.pending_defaults.len()));
        for (class, span) in self.pending_cases.split_off(cases_mark).into_iter().chain(defaults) {
            if !objects.contains_key(&class) {
                let range = class_ranges.get(&class).copied().unwrap_or(span);
                let companion = self.synthetic_companion(class, span, range, Vec::new());
                objects.insert(class, companion);
                companions.push(companion);
            }
        }
        companions
    }

    /// Whether a constructor of a class, its primary one (`clauses`) or a secondary one of its
    /// `body`, has a parameter with a default.
    fn constructor_has_default(&self, clauses: &[ParamClause], body: &[Stmt]) -> bool {
        let defaulted = |cls: &[ParamClause]| cls.iter().any(|cl| cl.params.iter().any(|p| p.default.is_some()));
        defaulted(clauses)
            || body.iter().any(|s| match s {
                Stmt::Def(d) => {
                    let def = self.ast.def(*d);
                    def.name == names::INIT && matches!(&def.kind, DefKind::Fun(f) if defaulted(&f.clauses))
                }
                _ => false,
            })
    }

    /// The companion object `Desugar.classDef` makes for a class that has none written, at the
    /// class's place and over its range (`companionDefs`' `withSpan(cdef.span)`), holding `body`.
    fn synthetic_companion(&mut self, name: Name, span: Span, range: Span, body: Vec<Stmt>) -> DefId {
        let class = ClassDef {
            kind: ClassKind::Object,
            tparams: Vec::new(),
            clauses: Vec::new(),
            parents: Vec::new(),
            body,
            exports: ListRef::EMPTY,
            self_type: None,
            self_alias: names::EMPTY,
        };
        let companion = self.ast.add_def(Def { name, span, mods: 0, annots: Vec::new(), kind: DefKind::Class(Box::new(class)) });
        self.ast.def_ranges[companion.idx()] = range;
        self.ast.synthetic_companions.push(companion);
        companion
    }

    fn derived_given(&mut self, pending: &PendingDerives, tc: TyExprId, tc_name: Name) -> DefId {
        let span = pending.span;
        let mut self_ty = self.ast.add_ty(TyExpr::Name(pending.class), span);
        if pending.is_object {
            self_ty = self.ast.add_ty(TyExpr::Singleton(self_ty), span);
        }
        if !pending.tparams.is_empty() {
            let mark = self.ty_scratch.len();
            for tp in &pending.tparams {
                let arg = self.ast.add_ty(TyExpr::Name(tp.name), span);
                self.ty_scratch.push(arg);
            }
            let args = self.ty_list(mark);
            self_ty = self.ast.add_ty(TyExpr::Apply(self_ty, args), span);
        }
        // `CanEqual` relates two types; scalac derives it for the class against itself.
        let binary = tc_name == names::CAN_EQUAL;
        let mark = self.ty_scratch.len();
        self.ty_scratch.push(self_ty);
        if binary {
            self.ty_scratch.push(self_ty);
        }
        let args = self.ty_list(mark);
        let ty = self.ast.add_ty(TyExpr::Apply(tc, args), span);
        let tparams = pending
            .tparams
            .iter()
            .map(|tp| TypeParam {
                name: tp.name,
                span: tp.span,
                variance: 0,
                arity: 0,
                hk_variances: Vec::new(),
                upper: tp.upper,
                lower: tp.lower,
                context_bounds: if binary { Vec::new() } else { vec![tc] },
                evidence_names: if binary { Vec::new() } else { vec![names::EMPTY] },
                annots: Vec::new(),
            })
            .collect();
        let alias = Some(self.ast.add_expr(Expr::Derived, span));
        let name = format!("derived${}", self.interner.get(tc_name));
        let name = self.interner.intern(&name);
        let given = GivenDef { tparams, clauses: Vec::new(), ty, alias, parents: Vec::new(), body: Vec::new(), self_alias: names::EMPTY };
        // Lazy as in scalac, so that the instance may depend on givens defined after it.
        let d = self.ast.add_def(Def { name, span, mods: mods::LAZY, annots: Vec::new(), kind: DefKind::Given(Box::new(given)) });
        self.ast.derived_givens.push(d);
        d
    }

    /// At a template body: `:` or `{` and the body's region, or a brace block where a parent is
    /// missing (`extends {`) or where a header's skip found it.
    fn at_template_body(&self) -> bool {
        (self.at(Tok::ColonEol) && self.kind_at(1) == Tok::Indent) || self.at_bare_body()
    }

    fn at_bare_body(&self) -> bool {
        self.at(Tok::Indent)
            && self.span().end > self.span().start
            && (self.pos == self.body_at || matches!(self.toks[self.pos - 1].kind, Tok::KwExtends | Tok::KwWith | Tok::Comma))
    }

    /// A template's statements, a language import among them enabling its feature until their
    /// end, as dotty restores `languageImportContext` after `templateStatSeq`; its case classes get
    /// the companions `Desugar` gives them (`add_derived_givens`), a class's or a trait's an object
    /// of each instance.
    fn parse_template_body(&mut self, is_enum: bool) -> (Vec<Stmt>, ListRef, Option<TyExprId>, Name) {
        let saved = self.syntax.postfix_ops;
        let body = self.parse_template_body_scoped(is_enum);
        self.syntax.postfix_ops = saved;
        body
    }

    /// Whether the parser stands at the placeholder of a script's statements in its wrapper class's body.
    fn at_script_body(&self) -> bool {
        self.script_end != usize::MAX && self.at(Tok::Ident) && self.tok().name == self.script_body
    }

    /// The script's statements, read from the file's start to its wrapper as the wrapper class's
    /// body at the placeholder (`script::wrap`), so that every position in them is the script's own.
    fn parse_script_body(&mut self, body: &mut Vec<Stmt>) {
        let (resume, end) = (self.pos + 1, self.script_end);
        self.script_end = usize::MAX;
        let wrapper = self.toks[end];
        self.toks[end].kind = Tok::Eof;
        self.pos = 0;
        self.statements(RecoverySite::TemplateBody, |p| {
            if p.at(Tok::KwExport) {
                p.parse_export();
            } else {
                p.parse_block_stmt(body);
            }
        });
        self.toks[end] = wrapper;
        self.pos = resume;
    }

    fn parse_template_body_scoped(&mut self, is_enum: bool) -> (Vec<Stmt>, ListRef, Option<TyExprId>, Name) {
        let mut body = Vec::new();
        let patdefs_mark = self.pattern_vals;
        if self.at_bare_body() {
            self.bump();
        } else if !(self.at(Tok::ColonEol) && self.kind_at(1) == Tok::Indent) {
            if self.at(Tok::ColonEol) {
                self.error_at(self.span(), "expected an indented body after ':'");
                self.skip_token(RecoverySite::ColonBody);
            }
            return (body, ListRef::EMPTY, None, names::EMPTY);
        } else {
            self.bump();
            self.bump();
        }
        let exports_mark = self.export_scratch.len();
        let derives_mark = self.pending_derives.len();
        let cases_mark = self.pending_cases.len();
        let defaults_mark = self.pending_defaults.len();
        let (alias_indent, self_type, alias) = self.parse_self_alias();
        let language = self.language_open.len();
        self.statements(RecoverySite::TemplateBody, |p| {
            let at_enum_case =
                |p: &Self| p.at(Tok::KwCase) && !matches!(p.kind_at(1), Tok::KwClass | Tok::KwObject);
            if is_enum && at_enum_case(p) {
                p.parse_enum_cases(&mut body, Vec::new());
            } else if is_enum && p.at(Tok::At) {
                let annots = p.parse_annots();
                if at_enum_case(p) {
                    p.parse_enum_cases(&mut body, annots);
                } else {
                    let mut defs = Vec::new();
                    let (start, first) = (p.span().start, p.ast.defs.len());
                    p.parse_annotated_def(annots, &mut defs);
                    p.close_def_ranges(first, start);
                    body.extend(defs.into_iter().map(Stmt::Def));
                }
            } else if p.at(Tok::KwExport) {
                p.parse_export();
            } else if p.at_script_body() {
                p.parse_script_body(&mut body);
            } else {
                p.parse_block_stmt(&mut body);
            }
        });
        if alias_indent {
            self.eat(Tok::Outdent);
        }
        self.end_region();
        self.close_language(language);
        self.desugar_pattern_vals(&mut body, patdefs_mark);
        // A case class of a class's or a trait's body has its companion there too, an object of
        // each instance (`Desugar.classDef`); an enum's cases are the enum's.
        if is_enum {
            self.pending_cases.truncate(cases_mark);
            self.pending_defaults.truncate(defaults_mark);
        }
        if self.pending_derives.len() > derives_mark || self.pending_cases.len() > cases_mark || self.pending_defaults.len() > defaults_mark {
            let defs: Vec<DefId> =
                body.iter().filter_map(|s| if let Stmt::Def(d) = s { Some(*d) } else { None }).collect();
            let companions = self.add_derived_givens(derives_mark, cases_mark, defaults_mark, &defs);
            self.place_companions(&mut body, companions, |s| if let Stmt::Def(d) = s { Some(*d) } else { None }, Stmt::Def);
        }
        (body, self.export_list(exports_mark), self_type, alias)
    }

    /// scalac's `makePatDef` for a member: `val (a, b) = e` is a private val holding
    /// `(e: @unchecked) match { case (a, b) => (a, b) }` and a val per variable reading its
    /// element; a lone variable is the match itself.
    fn desugar_pattern_vals(&mut self, body: &mut Vec<Stmt>, mark: u32) {
        if self.pattern_vals != mark {
            self.desugar_pattern_vals_in(body, true);
        }
    }

    /// `makePatDef` for a block's pattern definitions that `lowers_simple_tuple` names, as a member's:
    /// a lazy one's holder lazy and a `def` per variable (Desugar.scala 1659), a `SimpleTuple`'s
    /// holder and a val per variable.
    #[inline]
    pub(super) fn desugar_local_pattern_vals(&mut self, stmts: &mut Vec<Stmt>, mark: u32) {
        if self.pattern_vals != mark {
            self.desugar_pattern_vals_in(stmts, false);
        }
    }

    /// Whether `makePatDef`'s plan for a definition over `rhs` is a `SimpleTuple` over a holder
    /// (its right side's results tuple literals, Desugar.scala 1575, not split elementwise,
    /// 1626-1631), whose components are then the typer's plain selections `$1._i` (1650-1657),
    /// adapted as any selection is: a block lowers it here, as a member's is, and a lazy one (its
    /// variables `def`s), rather than leave it to the typer. The decision is the parsed tree's alone.
    fn lowers_simple_tuple(&self, rhs: ExprId, plan: Option<&(PatDefTuple, Vec<TupleComponent>)>) -> bool {
        match plan {
            Some((PatDefTuple::Simple, components)) => !(matches!(self.ast.expr(rhs), Expr::Tuple(_)) && components.iter().all(|c| c.name.is_some())),
            _ => false,
        }
    }

    /// `makePatDef` over a body's pattern definitions, a member's (`member`: each, the holder
    /// private) or a block's lazy ones. A lazy definition's holder is lazy and each variable a
    /// `def` reading its element (`DefDef(name, Nil, tpt, selector(idx))`, Desugar.scala 1659), so
    /// that each read adapts the element anew; above `MaxTupleArity` the element is
    /// `holder.apply(idx)`, zero-based (1650-1654).
    fn desugar_pattern_vals_in(&mut self, body: &mut Vec<Stmt>, member: bool) {
        // `makePatDef`'s plan of each pattern definition, decided once on the parsed definition, and
        // whether the body lowers it: a member each, a block its lazy ones and those
        // `lowers_simple_tuple` names, the others the typer's. A body that lowers none is left whole.
        let mut plans = Vec::new();
        for s in body.iter() {
            let Stmt::Def(d) = *s else { continue };
            let def = self.ast.def(d);
            let DefKind::Val { pat: Some(pat), ty, rhs: Some(rhs) } = def.kind else { continue };
            let plan = if ty.is_none() { self.ast.patdef_tuple(pat, rhs) } else { None };
            let lowered = member || def.mods & mods::LAZY != 0 || self.lowers_simple_tuple(rhs, plan.as_ref());
            plans.push((lowered, plan));
        }
        if !plans.iter().any(|&(lowered, _)| lowered) {
            return;
        }
        let capacity = body.len() + 2 * plans.len();
        let stmts = std::mem::replace(body, Vec::with_capacity(capacity));
        let mut plans = plans.into_iter();
        for stmt in stmts {
            let Stmt::Def(d) = stmt else {
                body.push(stmt);
                continue;
            };
            let DefKind::Val { pat: Some(pat), ty, rhs: Some(rhs) } = self.ast.def(d).kind else {
                body.push(stmt);
                continue;
            };
            let Some((true, plan)) = plans.next() else {
                body.push(stmt);
                continue;
            };
            let (span, m, annots) = {
                let def = self.ast.def(d);
                (def.span, def.mods, def.annots.clone())
            };
            let scope = self.ast.access_scopes.iter().find(|&&(at, _)| at == span.start).map(|&(_, s)| s);
            // The definitions made of it stand in its statement (`def_range`), not in what encloses
            // it, which a later `close_def_ranges` would give them.
            let range = self.ast.def_range(d);
            let define = |p: &mut Self, body: &mut Vec<Stmt>, name: Name, at: Span, mods: Mods, ty: Option<TyExprId>, rhs: ExprId| {
                let kind = DefKind::Val { pat: None, ty, rhs: Some(rhs) };
                let d = p.ast.add_def(Def { name, span: at, mods, annots: annots.clone(), kind });
                p.ast.def_ranges[d.idx()] = range;
                body.push(Stmt::Def(d));
                if let Some(scope) = scope.filter(|_| at.start != span.start) {
                    p.ast.access_scopes.push((at.start, scope));
                }
                d
            };
            // A variable's definition: a val, a `def` of a lazy definition's.
            let component = |p: &mut Self, body: &mut Vec<Stmt>, name: Name, at: Span, ty: Option<TyExprId>, rhs: ExprId| {
                if m & mods::LAZY == 0 {
                    return define(p, body, name, at, m, ty, rhs);
                }
                let fun = FunDef { tparams: Vec::new(), clauses: Vec::new(), ret: ty, body: Some(rhs), ext_tparams: 0, ext_clauses: 0, is_extension: false, ext_group: 0 };
                let d = p.ast.add_def(Def { name, span: at, mods: m & !mods::LAZY, annots: annots.clone(), kind: DefKind::Fun(Box::new(fun)) });
                p.ast.def_ranges[d.idx()] = range;
                body.push(Stmt::Def(d));
                if let Some(scope) = scope.filter(|_| at.start != span.start) {
                    p.ast.access_scopes.push((at.start, scope));
                }
                d
            };
            // The holder's `idx`-th element of `arity`: `_{idx + 1}`, or `apply(idx)` above
            // `MaxTupleArity`.
            let select = |p: &mut Self, holder: Name, idx: usize, arity: usize, at: Span| {
                let base = p.ast.add_expr(Expr::Ident(holder), at);
                if arity <= 22 {
                    let field = p.interner.intern(&format!("_{}", idx + 1));
                    return p.ast.add_expr(Expr::Select(base, field), at);
                }
                let fun = p.ast.add_expr(Expr::Select(base, names::APPLY), at);
                let index = p.ast.add_expr(Expr::IntLit(idx as i64), at);
                let args = push_list(&mut p.ast.expr_lists, &[index]);
                p.ast.add_expr(Expr::Apply(fun, args), at)
            };
            let holder_mods = if member { mods::PRIVATE | (m & mods::LAZY) } else { m & mods::LAZY };
            // dotty's tuple optimizations (`Ast::patdef_tuple`): the right side held as it is
            // (`SimpleTuple`) or the match binding the whole value (`MatchableTuple`), each named
            // component a val of its element of the type written for it, read where the pattern
            // closes (scalac's `$N._i`); a tuple literal of the arity with no wildcard and no type
            // over the pattern is one val per element.
            // A declared type (`val (a, b): T = e`) keeps the general lowering below: dotty hands
            // `makePatDef` the pattern ascribed it (`patDef`'s `Typed(pat, tpt)`), the ascription the
            // inner pattern's expected type, where teq's ascribed pattern is a type test.
            if let Some((kind, components)) = plan {
                let elems = match self.ast.expr(rhs) {
                    Expr::Tuple(l) if kind == PatDefTuple::Simple && ty.is_none() && components.iter().all(|c| c.name.is_some()) => Some(self.ast.expr_list(l).to_vec()),
                    _ => None,
                };
                if let Some(elems) = elems {
                    // 3.8.4's lazy split is a `def` per element over the elements themselves, each read
                    // evaluating it.
                    for (c, e) in components.iter().zip(elems) {
                        let d = component(self, body, c.name.unwrap(), c.span, c.ty, e);
                        self.ast.patdef_components.insert(d);
                    }
                    continue;
                }
                let lowered = match kind {
                    PatDefTuple::Simple => rhs,
                    PatDefTuple::Matchable => {
                        let scrutinee = self.ast.add_expr(Expr::Unchecked(rhs), span);
                        if !self.ast.is_unchecked(rhs) {
                            self.ast.patdef_scrutinees.insert(scrutinee);
                        }
                        let whole = self.fresh_name("");
                        let pat_span = self.ast.pat_spans[pat.idx()];
                        let bound = self.ast.add_pat(Pat::Bind(whole, Some(pat)), pat_span);
                        let result = self.ast.add_expr(Expr::Ident(whole), pat_span);
                        let cases = push_list(&mut self.ast.cases, &[CaseClause { pat: bound, guard: None, body: result }]);
                        self.ast.add_expr(Expr::Match(scrutinee, cases), span)
                    }
                };
                let holder = self.fresh_name("");
                let h = define(self, body, holder, span, holder_mods, ty, lowered);
                self.ast.patdef_holders.insert(h);
                let pat_span = self.ast.pat_spans[pat.idx()];
                let close = Span { start: pat_span.end.saturating_sub(1).max(pat_span.start), end: pat_span.end };
                for (i, c) in components.iter().enumerate() {
                    let Some(n) = c.name else { continue };
                    let element = select(self, holder, i, components.len(), close);
                    let d = component(self, body, n, c.span, c.ty, element);
                    self.ast.patdef_components.insert(d);
                }
                continue;
            }
            let mut vars = Vec::new();
            self.pattern_vars(pat, &mut vars);
            let typed = match ty {
                Some(t) => self.ast.add_expr(Expr::Typed(rhs, t), span),
                None => rhs,
            };
            let checked = !self.ast.is_unchecked(rhs);
            let scrutinee = self.ast.add_expr(Expr::Unchecked(typed), span);
            if checked {
                self.ast.patdef_scrutinees.insert(scrutinee);
            }
            let result = match vars.as_slice() {
                [] => self.ast.add_expr(Expr::UnitLit, span),
                [(n, s)] => self.ast.add_expr(Expr::Ident(*n), *s),
                _ => {
                    let items: Vec<ExprId> = vars.iter().map(|&(n, s)| self.ast.add_expr(Expr::Ident(n), s)).collect();
                    let l = push_list(&mut self.ast.expr_lists, &items);
                    self.ast.add_expr(Expr::Tuple(l), span)
                }
            };
            let cases = push_list(&mut self.ast.cases, &[CaseClause { pat, guard: None, body: result }]);
            let matched = self.ast.add_expr(Expr::Match(scrutinee, cases), span);
            if let [(n, s)] = vars.as_slice() {
                let d = define(self, body, *n, *s, m, None, matched);
                self.ast.patdef_components.insert(d);
                continue;
            }
            // scalac's `UniqueName` holder, `$N`, in no namespace of a parameter's (`x$N`).
            let holder = self.fresh_name("");
            let h = define(self, body, holder, span, holder_mods, None, matched);
            self.ast.patdef_holders.insert(h);
            for (i, &(n, s)) in vars.iter().enumerate() {
                let element = select(self, holder, i, vars.len(), s);
                let d = component(self, body, n, s, None, element);
                self.ast.patdef_components.insert(d);
            }
        }
    }

    /// A `given T` pattern binds a given in a case or a `for` alone, as scalac has it.
    fn reject_given_patterns(&mut self, p: PatId) {
        if self.ast.given_binds.contains(&p) {
            let span = self.ast.pat_spans[p.idx()];
            self.error_at(span, "given patterns are not allowed in a val definition,\nplease bind to an identifier and use an alias given.");
            return;
        }
        match self.ast.pat(p) {
            Pat::Bind(_, Some(inner)) | Pat::Typed(inner, _) | Pat::NamedField(_, inner) | Pat::Rest(inner) => self.reject_given_patterns(inner),
            Pat::Ctor(_, l) | Pat::Tuple(l) | Pat::Alt(l) | Pat::Interp(_, _, l) => {
                for q in self.ast.pat_list(l).to_vec() {
                    self.reject_given_patterns(q);
                }
            }
            _ => {}
        }
    }

    /// The variables a pattern binds, in order, with where each is written.
    fn pattern_vars(&self, p: PatId, out: &mut Vec<(Name, Span)>) {
        match self.ast.pat(p) {
            Pat::Bind(n, sub) => {
                out.push((n, self.ast.pat_spans[p.idx()]));
                if let Some(sub) = sub {
                    self.pattern_vars(sub, out);
                }
            }
            Pat::Typed(inner, _) | Pat::NamedField(_, inner) | Pat::Rest(inner) => self.pattern_vars(inner, out),
            Pat::Ctor(_, l) | Pat::Tuple(l) | Pat::Interp(_, _, l) => {
                for &q in self.ast.pat_list(l) {
                    self.pattern_vars(q, out);
                }
            }
            _ => {}
        }
    }

    /// `self =>` at the start of a template body names `this`, and `self: T =>` or `this: T =>`
    /// declares the self type `T` as well. A line break after the arrow opens an indentation
    /// region that ends with the body, which the caller closes when the first result is true.
    fn parse_self_alias(&mut self) -> (bool, Option<TyExprId>, Name) {
        self.skip_separators();
        let named = match self.kind() {
            Tok::Ident => true,
            Tok::KwThis => false,
            _ => return (false, None, names::EMPTY),
        };
        let typed = match self.kind_at(1) {
            Tok::Arrow => false,
            Tok::Colon if self.self_type_follows() => true,
            _ => return (false, None, names::EMPTY),
        };
        let alias = if named { self.tok().name } else { names::EMPTY };
        self.bump();
        let mut self_type = None;
        if typed {
            self.bump();
            let start = self.span();
            let mut t = self.parse_inter_type();
            while self.eat(Tok::KwWith) {
                let u = self.parse_inter_type();
                t = self.ast.add_ty(TyExpr::Inter(t, u), start.to(self.prev_span()));
            }
            self_type = Some(t);
        }
        self.expect(Tok::Arrow);
        (self.eat(Tok::Indent), self_type, alias)
    }

    fn self_type_follows(&self) -> bool {
        let mut i = 2;
        let mut depth = 0usize;
        loop {
            match self.kind_at(i) {
                Tok::LParen | Tok::LBracket => depth += 1,
                Tok::RParen | Tok::RBracket => depth = depth.saturating_sub(1),
                Tok::Arrow if depth == 0 => return true,
                Tok::Newline | Tok::Indent | Tok::Outdent | Tok::Eof | Tok::Eq => return false,
                _ => {}
            }
            i += 1;
        }
    }

    fn parse_enum_cases(&mut self, body: &mut Vec<Stmt>, annots: Vec<Annot>) {
        self.expect(Tok::KwCase);
        loop {
            let (name, span) = self.expect_ident();
            self.check_dollar_name(name, span);
            let first = self.ast.defs.len();
            let mark = self.error_events;
            let tparams = self.parse_type_params();
            let clauses = self.parse_param_clauses(Some(true), 0);
            let parents = self.parse_parents();
            let incomplete = if self.error_events == mark { 0 } else { mods::INCOMPLETE };
            let class = ClassDef {
                kind: ClassKind::EnumCase,
                tparams,
                clauses,
                parents,
                body: Vec::new(),
                exports: ListRef::EMPTY,
                self_type: None,
                self_alias: names::EMPTY,
            };
            let d = self.ast.add_def(Def {
                name,
                span,
                mods: mods::CASE | mods::FINAL | incomplete,
                annots: annots.clone(),
                kind: DefKind::Class(Box::new(class)),
            });
            self.close_def_ranges(first, span.start);
            if name != names::EMPTY {
                body.push(Stmt::Def(d));
            }
            if self.eat(Tok::Comma) {
                continue;
            }
            // `case A B`: a name on the same line is the next case, its comma missing.
            if !self.at(Tok::Ident) {
                break;
            }
            let (at, found) = self.found();
            self.error_at(at, format!("expected end of statement, found {}", found));
        }
    }

    /// True for `given name: ...`, `given name[T]: ...` and `given name(using ...): ...`, the
    /// last also where its `)` is missing.
    fn given_has_name(&self) -> bool {
        if !self.at(Tok::Ident) {
            return false;
        }
        if self.kind_at(1) == Tok::LParen && self.toks.get(self.pos + 2).is_some_and(|t| t.kind == Tok::Ident && t.name == names::USING) {
            return true;
        }
        let mut i = 1usize;
        let mut depth = 0usize;
        loop {
            match self.kind_at(i) {
                Tok::LBracket | Tok::LParen => depth += 1,
                Tok::RBracket | Tok::RParen => depth = depth.saturating_sub(1),
                Tok::Colon if depth == 0 => return true,
                Tok::ColonEol if depth == 0 && self.given_type_on_next_line(i) => return true,
                Tok::Newline if depth == 0 && self.param_clause_on_next_line(i) => {}
                Tok::Eof | Tok::Newline | Tok::Indent | Tok::Eq | Tok::KwWith | Tok::ColonEol
                    if depth == 0 =>
                {
                    return false
                }
                Tok::Eof => return false,
                _ => {}
            }
            i += 1;
        }
    }

    /// scalac's name for an anonymous given: the implemented type and the heads of its top-level
    /// arguments without their prefixes, so `Show[List[A]]` gives `given_Show_List`.
    pub(super) fn given_name_from_type(&mut self, ty: TyExprId) -> crate::intern::Name {
        let mut s = String::from("given");
        self.push_given_name_parts(ty, true, &mut s);
        self.interner.intern(&s)
    }

    fn push_given_name_parts(&self, ty: TyExprId, follow_args: bool, out: &mut String) {
        let node = self.ast.ty(ty);
        match node {
            TyExpr::Name(n) | TyExpr::Select(_, n) => {
                out.push('_');
                out.push_str(self.interner.get(n));
            }
            TyExpr::Apply(f, args) => {
                self.push_given_name_parts(f, follow_args, out);
                if follow_args {
                    for &a in self.ast.ty_list(args) {
                        self.push_given_name_parts(a, false, out);
                    }
                }
            }
            TyExpr::Tuple(items) => {
                for &t in self.ast.ty_list(items) {
                    self.push_given_name_parts(t, false, out);
                }
            }
            TyExpr::Fun(params, ret) if params.is_empty() => self.push_given_name_parts(ret, follow_args, out),
            TyExpr::Fun(params, ret) if follow_args => {
                for &p in self.ast.ty_list(params) {
                    self.push_given_name_parts(p, false, out);
                }
                out.push_str("_to");
                self.push_given_name_parts(ret, true, out);
            }
            TyExpr::Fun(..) => out.push_str("_Function"),
            TyExpr::Union(a, b) | TyExpr::Inter(a, b) => {
                out.push_str(if matches!(node, TyExpr::Union(..)) { "_|" } else { "_&" });
                if follow_args {
                    self.push_given_name_parts(a, false, out);
                    self.push_given_name_parts(b, false, out);
                }
            }
            TyExpr::Lambda(_, body) => self.push_given_name_parts(body, follow_args, out),
            _ => {}
        }
    }

    /// `given g:` at the end of a line names the given when the next line is the type, `X[T] =`
    /// or `X[T] with`, and is a given of the type `g` with a body otherwise, as scalac's
    /// `followingIsGivenDefWithColon` tells them.
    fn given_type_on_next_line(&self, colon: usize) -> bool {
        if self.kind_at(colon + 1) != Tok::Indent || !matches!(self.kind_at(colon + 2), Tok::Ident | Tok::OpIdent) {
            return false;
        }
        let mut i = colon + 3;
        let mut depth = 0usize;
        loop {
            match self.kind_at(i) {
                Tok::LBracket | Tok::LParen => depth += 1,
                Tok::RBracket | Tok::RParen if depth > 0 => depth -= 1,
                Tok::Eq | Tok::KwWith if depth == 0 => return true,
                // The type alone on its line, the `=` on the next (`given g:` / `Int` / `= 1`).
                Tok::Outdent if depth == 0 => return self.kind_at(i + 1) == Tok::Eq,
                _ if depth == 0 => return false,
                Tok::Eof => return false,
                _ => {}
            }
            i += 1;
        }
    }

    fn parse_given(&mut self, m: Mods, annots: Vec<Annot>) -> DefId {
        let start = self.span();
        self.expect(Tok::KwGiven);
        let mark = self.error_events;
        let mut named = None;
        if self.given_has_name() {
            let (n, s) = self.expect_ident();
            self.check_dollar_name(n, s);
            named = Some((n, s));
        }
        let has_signature = named.is_some()
            || (matches!(self.kind(), Tok::LBracket | Tok::LParen)
                && self.signature_followed_by_colon());
        // dotty's `hasEmbeddedColon`: parameters before the colon, the syntax before 3.6, which
        // turns 3.6's off for this given; and the source version's own rule (`newSyntaxAllowed`,
        // `SourceVersion.enablesNewGivens`).
        let old_syntax = if named.is_some() { matches!(self.kind(), Tok::LBracket | Tok::LParen) } else { has_signature };
        if old_syntax {
            self.migration(Migration::GivenSyntax, self.span(), "This old given syntax is no longer supported; use `=>` instead of `:`", crate::warnings::NO_ID);
        }
        let new_syntax = self.source.enables_new_givens() && !old_syntax;
        let mut tparams = Vec::new();
        let mut clauses = Vec::new();
        let mut region = None;
        let arrow_signature = |p: &Self, n: usize| {
            matches!(p.kind_at(n), Tok::LBracket | Tok::LParen) && p.group_followed_by_arrow(n)
        };
        if (named.is_some() && self.at(Tok::Colon) && arrow_signature(self, 1))
            || (named.is_none() && arrow_signature(self, 0))
        {
            // `given [A: Ord] => (other: Show[A]) => Ord[List[A]]`
            self.eat(Tok::Colon);
            while arrow_signature(self, 0) {
                if self.at(Tok::LBracket) {
                    tparams.extend(self.parse_type_params());
                } else {
                    let mark = clauses.len();
                    let lead = clauses.iter().map(|c: &ParamClause| c.params.len()).sum();
                    clauses.extend(self.parse_param_clauses(None, lead));
                    for c in &mut clauses[mark..] {
                        c.is_using = true;
                    }
                }
                self.expect(Tok::Arrow);
            }
        } else if has_signature {
            tparams = self.parse_type_params();
            let clauses_at = self.span();
            clauses = self.parse_param_clauses(None, 0);
            if clauses.iter().any(|c| !c.is_using) {
                self.error_at(clauses_at, "expected 'using': the parameters of a given are using clauses");
            }
            match self.eat_annotation_colon() {
                Some(r) => region = r,
                None => {
                    self.expect(Tok::Colon);
                }
            }
        }
        let mut ty_start = self.span();
        self.template_type = true;
        let ty = if has_signature {
            self.parse_type()
        } else {
            // dotty's `newTermParamssAndParents`: a type the parent position holds that `=>`
            // follows is a using clause of that one parameter (`given I[B] => J[B]`, `x$N` as
            // `typesToParams` names it), and so is a parameter group; the parent is what follows
            // the last arrow, a type of no arrow at its top (`implemented`).
            loop {
                if self.at(Tok::LParen) && arrow_signature(self, 0) {
                    let lead = clauses.iter().map(|c: &ParamClause| c.params.len()).sum();
                    let mark = clauses.len();
                    clauses.extend(self.parse_param_clauses(None, lead));
                    for c in &mut clauses[mark..] {
                        c.is_using = true;
                    }
                    self.expect(Tok::Arrow);
                    ty_start = self.span();
                    continue;
                }
                let t = if self.at(Tok::LBracket) || self.at(Tok::Arrow) { self.parse_type() } else { self.parse_union_type() };
                if !self.at(Tok::Arrow) {
                    break t;
                }
                let span = ty_start.to(self.prev_span());
                self.bump();
                let n = clauses.iter().map(|c: &ParamClause| c.params.len()).sum::<usize>() + 1;
                let name = self.interner.intern(&format!("x${}", n));
                clauses.push(ParamClause { params: vec![Param { name, span, ty: t, default: None, mods: 0, annots: ListRef::EMPTY }], is_using: true, is_implicit: false });
                ty_start = self.span();
            }
        };
        self.template_type = false;
        let ty_span = ty_start.to(self.prev_span());
        let (name, span) = match named {
            Some((n, s)) => (n, s),
            None if matches!(self.ast.ty(ty), TyExpr::Error) => (names::EMPTY, start.to(ty_span)),
            None => (self.given_name_from_type(ty), start.to(ty_span)),
        };
        let m = if self.error_events == mark { m } else { m | mods::INCOMPLETE };
        let mut alias = None;
        let mut parents: Vec<Parent> = Vec::new();
        let mut body = Vec::new();
        let mut self_alias = names::EMPTY;
        // `given x: T` with nothing after the type is an old-style abstract given, an abstract
        // def flagged `Given` (dotty's `Parsers.givenDef`, 4586-4602); an anonymous one stays a
        // structural instance, as under 3.8's syntax. An `=` starting the next line continues the
        // given (`given g:` / `Long` / `= 5L`).
        let abstract_decl = named.is_some()
            && matches!(self.kind(), Tok::Newline | Tok::Semi | Tok::Outdent | Tok::RBrace | Tok::Eof)
            && !(matches!(self.kind(), Tok::Newline | Tok::Outdent) && self.kind_at(1) == Tok::Eq);
        if abstract_decl {
            // Nothing follows the type.
        } else if self.at(Tok::LParen) {
            // `given C(args)` is a structural instance whose parent is the constructor call, with
            // or without a body (dotty's `Parsers.givenDef`, 4610-4623: a template over
            // `constrApps`, a `ModuleDef` where the given has no parameters, a class beside a
            // def where it has): the given's class of its own extends `C`, which may be a trait
            // (`given QC()`), and the others the clause names.
            let mut args = Vec::new();
            let mut ends = Vec::new();
            while self.at(Tok::LParen) {
                args.push(self.parse_args());
                ends.push(self.prev_span().end);
            }
            parents.push(Parent { ty, args, ends });
            // More parents after a comma under 3.6's syntax, after `with` otherwise
            // (`moreConstrApps`).
            while new_syntax && self.eat(Tok::Comma) {
                self.template_type = true;
                let pty = self.parse_simple_type();
                self.template_type = false;
                let mut pargs = Vec::new();
                let mut ends = Vec::new();
                while self.at(Tok::LParen) {
                    pargs.push(self.parse_args());
                    ends.push(self.prev_span().end);
                }
                parents.push(Parent { ty: pty, args: pargs, ends });
            }
            while self.at(Tok::KwWith) && !matches!(self.kind_at(1), Tok::LBrace | Tok::Indent | Tok::Newline) {
                self.bump();
                self.template_type = true;
                let pty = self.parse_simple_type();
                self.template_type = false;
                let mut pargs = Vec::new();
                let mut ends = Vec::new();
                while self.at(Tok::LParen) {
                    pargs.push(self.parse_args());
                    ends.push(self.prev_span().end);
                }
                parents.push(Parent { ty: pty, args: pargs, ends });
            }
            // The body after `with` where 3.6's syntax is off or the given has no parameters,
            // after a colon or in braces otherwise (`Parsers.givenDef`'s `withTemplate` and
            // `templateBodyOpt`).
            let with_template = !new_syntax || (self.at(Tok::KwWith) && tparams.is_empty() && clauses.is_empty());
            if with_template && !self.given_ends_here() {
                self.given_with_template();
                if !self.eat(Tok::KwWith) {
                    let msg = if self.at(Tok::ColonEol) { "'with' expected, but ':' found" } else { "'with' expected" };
                    self.error_at(self.span(), msg);
                } else if self.eat(Tok::Indent) {
                    let (alias_indent, _, named) = self.parse_self_alias();
                    self_alias = named;
                    body = self.parse_given_body();
                    if alias_indent {
                        self.eat(Tok::Outdent);
                    }
                } else if self.at(Tok::LBrace) {
                    let (stmts, exports, _, named) = self.parse_template_body(false);
                    body = stmts;
                    self_alias = named;
                    if !exports.is_empty() {
                        self.error_at(ty_start, "export clauses are not supported in a given");
                    }
                }
            } else if self.at(Tok::KwWith) {
                self.error_at(self.span(), "end of statement expected but 'with' found");
            } else {
                let (stmts, exports, _, named) = self.parse_template_body(false);
                body = stmts;
                self_alias = named;
                if !exports.is_empty() {
                    self.error_at(ty_start, "export clauses are not supported in a given");
                }
            }
        } else if {
            self.annotation_before_eq(region);
            self.eat(Tok::Eq)
        } {
            alias = Some(self.parse_rhs(region));
        } else if self.at(Tok::KwWith) {
            // `with` opens the body where 3.6's syntax is off or the given has no parameters
            // (`Parsers.givenDef`'s `withTemplate`).
            if new_syntax && !(tparams.is_empty() && clauses.is_empty()) {
                self.error_at(self.span(), "end of statement expected but 'with' found");
            } else {
                self.given_with_template();
            }
            self.bump();
            if self.eat(Tok::Indent) {
                let (alias_indent, _, named) = self.parse_self_alias();
                self_alias = named;
                body = self.parse_given_body();
                if alias_indent {
                    self.eat(Tok::Outdent);
                }
            }
        } else if self.at(Tok::ColonEol) {
            // A body after a colon is 3.6's syntax (`possibleTemplateStart`).
            if !new_syntax {
                self.error_at(self.span(), "'with' expected, but ':' found");
            }
            let exports_at = self.span();
            let (stmts, exports, _, named) = self.parse_template_body(false);
            body = stmts;
            self_alias = named;
            if !exports.is_empty() {
                self.error_at(exports_at, "export clauses are not supported in a given");
            }
        }
        self.end_annotation_region(region);
        if abstract_decl {
            // Under 3.6's syntax the type alone is reported by the version, past it
            // (`Parsers.givenDef`, 4597-4601).
            if new_syntax {
                let past = Span::new(self.prev_span().end, self.prev_span().end);
                self.migration(
                    Migration::GivenSyntax,
                    past,
                    "This defines an abstract given, which is no longer supported. Use a `deferred` given instead.\nOr, if you intend to define a concrete given, follow the type with `()` arguments.",
                    crate::warnings::NO_ID,
                );
            }
            let fun = FunDef { tparams, clauses, ret: Some(ty), body: None, ext_tparams: 0, ext_clauses: 0, is_extension: false, ext_group: 0 };
            return self.ast.add_def(Def { name, span, mods: m | mods::GIVEN, annots, kind: DefKind::Fun(Box::new(fun)) });
        }
        let given = GivenDef { tparams, clauses, ty, alias, parents, body, self_alias };
        let m = if named.is_none() { m | mods::ANONYMOUS } else { m };
        self.ast.add_def(Def { name, span, mods: m, annots, kind: DefKind::Given(Box::new(given)) })
    }

    /// A given's body after `with`, reported by the version where it begins (`Parsers.withTemplate`).
    fn given_with_template(&mut self) {
        self.migration(
            Migration::GivenSyntax,
            self.span(),
            "Given member definitions starting with `with` are no longer supported; use `{...}` or `:` followed by newline instead",
            crate::warnings::NO_ID,
        );
    }

    /// Whether nothing of the given follows: a statement's end.
    fn given_ends_here(&self) -> bool {
        matches!(self.kind(), Tok::Newline | Tok::Semi | Tok::Outdent | Tok::RBrace | Tok::Eof)
    }

    fn parse_given_body(&mut self) -> Vec<Stmt> {
        let mut body = Vec::new();
        self.statements(RecoverySite::GivenBody, |p| p.parse_block_stmt(&mut body));
        self.end_region();
        body
    }

    fn group_followed_by_arrow(&self, from: usize) -> bool {
        let mut i = from;
        let mut depth = 0usize;
        loop {
            match self.kind_at(i) {
                Tok::LBracket | Tok::LParen | Tok::LBrace => depth += 1,
                Tok::RBracket | Tok::RParen | Tok::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return self.kind_at(i + 1) == Tok::Arrow;
                    }
                }
                Tok::Eof => return false,
                _ => {}
            }
            i += 1;
        }
    }

    /// For anonymous givens: `given [T](using Ord[T]): Ord[List[T]]`.
    fn signature_followed_by_colon(&self) -> bool {
        let mut i = 0usize;
        let mut depth = 0usize;
        loop {
            match self.kind_at(i) {
                Tok::LBracket | Tok::LParen => depth += 1,
                Tok::RBracket | Tok::RParen => {
                    depth -= 1;
                    if depth == 0 && self.param_clause_on_next_line(i + 1) {
                        i += 1;
                    } else if depth == 0 && !matches!(self.kind_at(i + 1), Tok::LParen | Tok::LBracket) {
                        return self.kind_at(i + 1) == Tok::Colon;
                    }
                }
                Tok::Eof => return false,
                _ => {}
            }
            i += 1;
        }
    }
}
