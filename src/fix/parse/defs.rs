//! ScalametaParser.scala's sections on modifiers and annotations, parameters, definitions (imports and exports among
//! them), constructors, self and templates.

use super::stats::{Location, StatPf};
use super::{Owner, Parser, Stop, P};
use crate::fix::nodes::{F, K};
use crate::fix::scanner::C;

/// `GivenSig`: a given's name, its clause groups, its declared type and where that type starts.
struct GivenSig {
    name: u32,
    pcg: Vec<u32>,
    decl_type: Option<u32>,
    decl_pos: Option<i32>,
}

impl GivenSig {
    fn named(name: u32) -> GivenSig {
        GivenSig { name, pcg: Vec::new(), decl_type: None, decl_pos: None }
    }
}

impl Owner {
    /// `isPrimaryCtorAllowed`, a trait's under `allowTraitParameters`.
    fn is_primary_ctor_allowed(self, dialect: &super::Dialect) -> bool {
        match self {
            Owner::Trait => dialect.allow_trait_parameters,
            Owner::Object | Owner::Given => false,
            Owner::Class | Owner::Enum => true,
        }
    }
    fn is_secondary_ctor_allowed(self) -> bool {
        matches!(self, Owner::Class | Owner::Enum)
    }
    /// The owner's class name less `OwnedBy` (`getSimpleName` of a Scala object, its `$` kept).
    fn what(self) -> &'static str {
        match self {
            Owner::Class => "Class$",
            Owner::Trait => "Trait$",
            Owner::Object => "Object$",
            Owner::Enum => "Enum$",
            Owner::Given => "Given$",
        }
    }
}

impl<'a> Parser<'a> {
    /* -------- MODIFIERS and ANNOTATIONS -------- */

    /// `accessModifier(mod)`: `private` or `protected`, its qualifier in brackets.
    fn access_modifier(&mut self, kind: K) -> P<u32> {
        self.auto_pos(|p| {
            p.next();
            let within = if !p.accept_opt(C::LeftBracket) {
                p.anon_name()
            } else {
                let within = if p.at(C::KwThis) {
                    p.anon_this()
                } else {
                    let n = p.term_name()?;
                    p.indeterminate(n)
                };
                p.accept(C::RightBracket)?;
                within
            };
            Ok(p.mk(kind, vec![(F::Within, within)]))
        })
    }

    /// `badModifier(tok, isLocal)`.
    fn bad_modifier<T>(&self, is_local: bool) -> P<T> {
        self.refused(format!("{}modifier expected but {} found", if is_local { "local " } else { "" }, self.curr_text()))
    }

    /// `atCurPosNext(Mod.X())`.
    pub(super) fn mod_at_cur(&mut self, kind: K) -> u32 {
        let m = self.mk(kind, Vec::new());
        self.at_cur_pos_next(m)
    }

    /// `modifier(tok, isLocal)`, a modifier keyword's and a soft modifier's (each under its dialect flag).
    fn modifier(&mut self, is_local: bool) -> P<u32> {
        let kind = match self.curr_c() {
            C::KwAbstract => K::ModAbstract,
            C::KwFinal => K::ModFinal,
            C::KwSealed => K::ModSealed,
            C::KwImplicit => K::ModImplicit,
            C::KwLazy => K::ModLazy,
            C::KwOverride if !is_local => K::ModOverride,
            C::KwPrivate if !is_local => return self.access_modifier(K::ModPrivate),
            C::KwProtected if !is_local => return self.access_modifier(K::ModProtected),
            C::Ident => {
                let d = self.dialect;
                match self.curr_text() {
                    "inline" if d.allow_inline_mods => K::ModInline,
                    "infix" if d.allow_infix_mods => K::ModInfix,
                    "into" if d.allow_parameter_type_conversions => K::ModInto,
                    "open" if d.allow_open_class && !is_local => K::ModOpen,
                    "opaque" if d.allow_opaque_types => K::ModOpaque,
                    "transparent" if d.allow_inline_mods => K::ModTransparent,
                    "erased" if d.allow_erased_defs => K::ModErased,
                    "tracked" if d.allow_tracked_parameters => K::ModTracked,
                    _ => return self.bad_modifier(is_local),
                }
            }
            _ => return self.bad_modifier(is_local),
        };
        Ok(self.mod_at_cur(kind))
    }

    /// `ctorModifiers(buf)`.
    fn ctor_modifiers(&mut self, buf: &mut Vec<u32>) -> P<()> {
        match self.curr_c() {
            C::KwPrivate => buf.push(self.access_modifier(K::ModPrivate)?),
            C::KwProtected => buf.push(self.access_modifier(K::ModProtected)?),
            _ => {}
        }
        Ok(())
    }

    /// `tparamModifiers(buf)`: `+` or `-`.
    fn tparam_modifiers(&mut self, buf: &mut Vec<u32>) {
        if self.at(C::Ident) {
            match self.curr_text() {
                "+" => buf.push(self.mod_at_cur(K::ModCovariant)),
                "-" => buf.push(self.mod_at_cur(K::ModContravariant)),
                _ => {}
            }
        }
    }

    /// `modifiersBuf(buf, isLocal, isParams)`.
    pub(super) fn modifiers_buf(&mut self, buf: &mut Vec<u32>, is_local: bool, is_params: bool) -> P<()> {
        loop {
            let c = self.curr_c();
            if c.at_eol() && !is_local {
                self.next();
            } else if c == C::KwCase && matches!(self.peek_c(), C::KwObject | C::KwClass) {
                buf.push(self.mod_at_cur(K::ModCase));
                return Ok(());
            } else if c.modifier_keyword() || c == C::Ident && self.is_soft_modifier_here(is_params) {
                let m = self.modifier(is_local)?;
                buf.push(m);
                self.new_lines_opt();
            } else {
                return Ok(());
            }
        }
    }

    /// `ParamsModifier` where parameters are read, `isSoftModifier` elsewhere.
    fn is_soft_modifier_here(&self, is_params: bool) -> bool {
        if is_params {
            let d = self.dialect;
            !self.peek(C::Colon) && (self.at_soft(d.allow_inline_mods, "inline") || self.at_soft(d.allow_erased_defs, "erased") || self.at_soft(d.allow_tracked_parameters, "tracked"))
        } else {
            self.s.is_soft_modifier(self.curr_index())
        }
    }

    /// `annots(skipNewLines, allowArgss)`.
    pub(super) fn annots(&mut self, skip_new_lines: bool, allow_argss: bool) -> P<Vec<u32>> {
        let mut buf = Vec::new();
        self.annots_buf(&mut buf, skip_new_lines, false, allow_argss)?;
        Ok(buf)
    }

    /// `annotsBuf(annots, skipNewLines, insidePrimaryCtorAnnot, allowArgss)`.
    pub(super) fn annots_buf(&mut self, buf: &mut Vec<u32>, skip_new_lines: bool, inside_primary_ctor_annot: bool, allow_argss: bool) -> P<()> {
        while self.at(C::At) {
            self.next();
            let start = self.prev_index();
            let annot = self.auto_end_pos(start, |p| {
                let start = p.curr_index();
                let tpe = p.simple_type()?;
                let init = p.init_rest_at(start, tpe, allow_argss, inside_primary_ctor_annot, false)?;
                Ok(p.mk(K::ModAnnot, vec![(F::Init, init)]))
            })?;
            buf.push(annot);
            if skip_new_lines {
                self.new_line_opt();
            }
        }
        Ok(())
    }

    /* -------- PARAMETERS -------- */

    fn param_clause_group(&mut self, tparams: u32, clauses: Vec<u32>) -> u32 {
        let mut fields = vec![(F::TparamClause, tparams)];
        fields.extend(clauses.into_iter().map(|c| (F::ParamClauses, c)));
        self.mk(K::ParamClauseGroup, fields)
    }

    /// `memberParamClauseGroupOnParen()`.
    fn member_param_clause_group_on_paren(&mut self) -> P<u32> {
        let tparams = self.empty_type_params();
        self.auto_pos(|p| {
            let clauses = p.term_param_clauses_on_paren()?;
            Ok(p.param_clause_group(tparams, clauses))
        })
    }

    /// `memberParamClauseGroupOnBracket()`.
    fn member_param_clause_group_on_bracket(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            let tparams = p.type_param_clause_on_bracket()?;
            let clauses = p.term_param_clauses()?;
            Ok(p.param_clause_group(tparams, clauses))
        })
    }

    /// `memberParamClauseGroup(isFirst)`.
    pub(super) fn member_param_clause_group(&mut self, is_first: bool) -> P<Option<u32>> {
        Ok(match self.curr_c() {
            C::LeftParen if is_first => Some(self.member_param_clause_group_on_paren()?),
            C::LeftBracket => Some(self.member_param_clause_group_on_bracket()?),
            c if c.eol() => match self.peek_c() {
                C::LeftParen if is_first => {
                    self.next();
                    Some(self.member_param_clause_group_on_paren()?)
                }
                C::LeftBracket => {
                    self.next();
                    Some(self.member_param_clause_group_on_bracket()?)
                }
                _ => None,
            },
            _ => None,
        })
    }

    /// `memberParamClauseGroups()`: groups while the last clause read is not `implicit`.
    fn member_param_clause_groups(&mut self) -> P<Vec<u32>> {
        let mut buf = Vec::new();
        while let Some(pcg) = self.member_param_clause_group(buf.is_empty())? {
            buf.push(pcg);
            let last = self.nodes[pcg as usize].fields.iter().rev().find(|&&(f, _)| f == F::ParamClauses).map(|&(_, c)| c);
            if !last.is_some_and(|pc| !self.clause_mod_is(pc, K::ModImplicit)) {
                break;
            }
        }
        Ok(buf)
    }

    fn clause_mod_is(&self, pc: u32, kind: K) -> bool {
        self.nodes[pc as usize].fields.iter().any(|&(f, m)| f == F::Mod && self.kind(m) == kind)
    }

    /// `termParamClauses()`.
    pub(super) fn term_param_clauses(&mut self) -> P<Vec<u32>> {
        if !self.is_after_opt_new_line(C::LeftParen) { Ok(Vec::new()) } else { self.term_param_clauses_on_paren() }
    }

    /// `termParamClausesOnParen()`.
    fn term_param_clauses_on_paren(&mut self) -> P<Vec<u32>> {
        let mut paramss = Vec::new();
        loop {
            paramss.push(self.term_param_clause_on_paren()?);
            if !self.is_after_opt_new_line(C::LeftParen) {
                return Ok(paramss);
            }
        }
    }

    /// `termParamClauseOnParen()`: its `implicit` or `using` the clause's `mod` and every parameter's.
    pub(super) fn term_param_clause_on_paren(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            p.next();
            if p.accept_opt(C::RightParen) {
                return Ok(p.mk(K::ParamClause, Vec::new()));
            }
            let mod_ = match p.curr_c() {
                C::KwImplicit => Some(p.mod_at_cur(K::ModImplicit)),
                C::Ident if !p.peek(C::Colon) && p.at_soft(p.dialect.allow_given_using, "using") => Some(p.mod_at_cur(K::ModUsing)),
                _ => None,
            };
            let mut params = vec![p.term_param(mod_)?];
            while p.accept_opt(C::Comma) {
                params.push(p.term_param(mod_)?);
            }
            let clause_mod = mod_.or_else(|| p.get_mod(&params));
            let mut fields: Vec<(F, u32)> = params.into_iter().map(|x| (F::Values, x)).collect();
            fields.extend(clause_mod.map(|m| (F::Mod, m)));
            let clause = p.mk(K::ParamClause, fields);
            p.in_parens_after_open(clause)
        })
    }

    /// `Term.ParamClause.getMod(v)`: the first parameter's `using`, or its `implicit` where every other has one.
    pub(super) fn get_mod(&self, params: &[u32]) -> Option<u32> {
        let mods = |x: u32| self.nodes[x as usize].fields.iter().filter(|&&(f, _)| f == F::Mods).map(|&(_, m)| m).collect::<Vec<_>>();
        let (&head, tail) = params.split_first()?;
        let first = *mods(head).first()?;
        match self.kind(first) {
            K::ModUsing => Some(first),
            K::ModImplicit if tail.iter().all(|&x| mods(x).iter().any(|&m| self.kind(m) == K::ModImplicit)) => Some(first),
            _ => None,
        }
    }

    /// `termParam(mod)`.
    fn term_param(&mut self, mod_: Option<u32>) -> P<u32> {
        self.auto_pos(|p| {
            let mut mods = Vec::new();
            p.annots_buf(&mut mods, false, false, true)?;
            let num_annots = mods.len();
            p.modifiers_buf(&mut mods, false, true)?;
            let has_explicit_mods = mods[num_annots..].iter().any(|&m| match p.kind(m) {
                K::ModErased | K::ModInline => false,
                K::ModPrivate | K::ModProtected => p.children(m).any(|w| p.kind(w) == K::NameAnonymous),
                _ => true,
            });
            if let Some(m) = mod_ {
                let kind = p.kind(m);
                if mods.iter().all(|&x| p.kind(x) != kind) {
                    mods.insert(0, m);
                }
            }
            match p.curr_c() {
                C::KwVal => mods.push(p.mod_at_cur(K::ModValParam)),
                C::KwVar => mods.push(p.mod_at_cur(K::ModVarParam)),
                _ if has_explicit_mods => return p.expected_c(C::KwVal),
                _ => {}
            }
            let (name, tpt) = if !p.peek(C::Colon) {
                let name = p.anon_name();
                (name, p.param_type()?)
            } else if p.at(C::Ident) {
                let name = p.term_name()?;
                p.accept(C::Colon)?;
                (name, p.param_type()?)
            } else {
                return p.expected_c(C::Ident);
            };
            let mut fields: Vec<(F, u32)> = mods.into_iter().map(|m| (F::Mods, m)).collect();
            fields.push((F::Name, name));
            fields.push((F::Decltpe, tpt));
            if p.accept_opt(C::Equals) {
                fields.push((F::Default, p.expr()?));
            }
            Ok(p.mk(K::Param, fields))
        })
    }

    /// `emptyTypeParams`: an empty `Type.ParamClause` at the current token.
    pub(super) fn empty_type_params(&mut self) -> u32 {
        let n = self.mk(K::TypeParamClause, Vec::new());
        self.at_cur_pos_empty(n)
    }

    /// `typeParamClauseOpt()`.
    pub(super) fn type_param_clause_opt(&mut self) -> P<u32> {
        if !self.is_after_opt_new_line(C::LeftBracket) { Ok(self.empty_type_params()) } else { self.type_param_clause_on_bracket() }
    }

    /// `typeParamClauseOnBracket()`: `typeClauseInBrackets(typeParam(), Type.ParamClause)`.
    pub(super) fn type_param_clause_on_bracket(&mut self) -> P<u32> {
        self.type_clause_in_brackets(K::TypeParamClause, |p| p.type_param())
    }

    /// `typeParam()`.
    fn type_param(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            let mut mods = Vec::new();
            p.annots_buf(&mut mods, false, false, true)?;
            p.tparam_modifiers(&mut mods);
            let name = match p.curr_c() {
                C::Ident if p.dialect.allow_capture_checking && p.peek_c() == C::Ident && p.peek_text() == "^" => return Err(Stop::NotYet("Type.CapSetName")),
                C::Ident => p.type_name()?,
                C::Underscore => p.name_placeholder(),
                _ => return p.refused("identifier expected".to_string()),
            };
            let tparams = p.type_param_clause_opt()?;
            let bounds = p.type_bounds()?;
            let mut fields: Vec<(F, u32)> = mods.into_iter().map(|m| (F::Mods, m)).collect();
            fields.extend([(F::Name, name), (F::TparamClause, tparams), (F::Bounds, bounds)]);
            Ok(p.mk(K::TypeParam, fields))
        })
    }

    /// `contextBoundOrAlias(allowAlias)`: a context bound, `A as B` an alias where allowed.
    fn context_bound_or_alias(&mut self, allow_alias: bool) -> P<u32> {
        let t = self.typ()?;
        let name = |p: &Self, f: F| p.field(t, f).filter(|&n| p.kind(n) == K::TypeName);
        if allow_alias && self.kind(t) == K::TypeApplyInfix {
            if let (Some(nm), Some(op), Some(alias)) = (name(self, F::Lhs), name(self, F::Op), name(self, F::Rhs)) {
                if self.nodes[op as usize].value.as_deref() == Some("as") {
                    let b = self.mk(K::TypeBoundsAlias, vec![(F::Name, alias), (F::Bounds, nm)]);
                    return Ok(self.copy_pos(t, b));
                }
            }
        }
        Ok(t)
    }

    /// `typeBounds()`, `typeBoundsWithOptViewBounds()`.
    pub(super) fn type_bounds(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            let lo = p.bound(C::Supertype)?;
            let hi = p.bound(C::Subtype)?;
            let mut view = Vec::new();
            while p.accept_opt(C::Viewbound) {
                view.push(p.context_bound_or_alias(false)?);
            }
            let mut context = Vec::new();
            if p.accept_opt(C::Colon) {
                if p.accept_opt(C::LeftBrace) {
                    loop {
                        context.push(p.context_bound_or_alias(p.dialect.allow_improved_type_classes_syntax)?);
                        if !p.accept_opt(C::Comma) {
                            break;
                        }
                    }
                    p.in_braces_after_open(())?;
                } else {
                    loop {
                        context.push(p.context_bound_or_alias(p.dialect.allow_improved_type_classes_syntax)?);
                        if !p.accept_opt(C::Colon) {
                            break;
                        }
                    }
                }
            }
            let mut fields = Vec::new();
            fields.extend(lo.map(|t| (F::Lo, t)));
            fields.extend(hi.map(|t| (F::Hi, t)));
            fields.extend(context.into_iter().map(|t| (F::Context, t)));
            fields.extend(view.into_iter().map(|t| (F::View, t)));
            Ok(p.mk(K::TypeBounds, fields))
        })
    }

    /// `bound[T]`.
    fn bound(&mut self, c: C) -> P<Option<u32>> {
        if self.accept_opt(c) { Ok(Some(self.typ()?)) } else { Ok(None) }
    }

    /* -------- DEFS -------- */

    /// `exportStmt()`.
    pub(super) fn export_stmt(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            p.accept(C::KwExport)?;
            let importers = p.importers()?;
            Ok(p.mk(K::Export, importers))
        })
    }

    /// `importStmt()`.
    pub(super) fn import_stmt(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            p.accept(C::KwImport)?;
            let importers = p.importers()?;
            Ok(p.mk(K::Import, importers))
        })
    }

    /// `commaSeparated(importer())`.
    fn importers(&mut self) -> P<Vec<(F, u32)>> {
        let mut out = vec![(F::Importers, self.importer()?)];
        while self.accept_opt(C::Comma) {
            out.push((F::Importers, self.importer()?));
        }
        Ok(out)
    }

    fn importer_of(&mut self, sid: u32, importees: Vec<u32>) -> u32 {
        let mut fields = vec![(F::Ref, sid)];
        fields.extend(importees.into_iter().map(|i| (F::Importees, i)));
        self.mk(K::Importer, fields)
    }

    /// `importer()`.
    fn importer(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            let sid = p.stable_id()?;
            let select = (p.kind(sid) == K::TermSelect).then(|| (p.field(sid, F::Qual), p.field(sid, F::Name)));
            match select {
                Some((Some(qual), Some(tn))) if p.kind(tn) == K::TermName && p.is_path(qual) => {
                    if p.accept_opt(C::Dot) {
                        let importees = p.importees()?;
                        Ok(p.importer_of(sid, importees))
                    } else if p.at_soft(p.dialect.allow_as_for_import_rename, "as") {
                        p.next();
                        let from = p.indeterminate(tn);
                        let rename = p.importee_rename(from)?;
                        Ok(p.importer_of(qual, vec![rename]))
                    } else if p.dialect.allow_star_wildcard_import && p.s.text_of(p.beg_index(tn)) == "*" {
                        let w = p.mk(K::ImporteeWildcard, Vec::new());
                        let w = p.copy_pos(tn, w);
                        Ok(p.importer_of(qual, vec![w]))
                    } else {
                        let name = p.indeterminate(tn);
                        let i = p.mk(K::ImporteeName, vec![(F::Name, name)]);
                        let i = p.copy_pos(tn, i);
                        Ok(p.importer_of(qual, vec![i]))
                    }
                }
                _ if p.kind(sid) == K::TermName && p.at_soft(p.dialect.allow_as_for_import_rename, "as") => {
                    p.next();
                    let anon = p.mk(K::TermAnonymous, Vec::new());
                    let anon = p.at_cur_pos_empty(anon);
                    let from = p.indeterminate(sid);
                    let rename = p.importee_rename(from)?;
                    Ok(p.importer_of(anon, vec![rename]))
                }
                _ => {
                    p.accept(C::Dot)?;
                    let importees = p.importees()?;
                    Ok(p.importer_of(sid, importees))
                }
            }
        })
    }

    /// `isPath` (trees' internal package).
    fn is_path(&self, t: u32) -> bool {
        match self.kind(t) {
            K::TermThis | K::TermName | K::TermAnonymous => true,
            K::TermSelect => self.field(t, F::Qual).is_some_and(|q| self.kind(q) == K::TermSuper || self.is_path(q)),
            _ => false,
        }
    }

    /// `importees()`: under `allowGivenImports` a name `given` beside a wildcard its `Importee.GivenAll`.
    fn importees(&mut self) -> P<Vec<u32>> {
        if !self.accept_opt(C::LeftBrace) {
            return Ok(vec![self.import_wildcard_or_name()?]);
        }
        let mut importees = vec![self.importee()?];
        while self.accept_opt(C::Comma) {
            importees.push(self.importee()?);
        }
        self.in_braces_after_open(())?;
        if self.dialect.allow_given_imports && importees.iter().any(|&i| self.kind(i) == K::ImporteeWildcard) {
            for i in importees.iter_mut() {
                let name = self.field(*i, F::Name);
                if self.kind(*i) == K::ImporteeName && name.is_some_and(|n| self.nodes[n as usize].value.as_deref() == Some("given")) {
                    let all = self.mk(K::ImporteeGivenAll, Vec::new());
                    *i = self.copy_pos(name.unwrap(), all);
                }
            }
        }
        Ok(importees)
    }

    /// `Wildcard()`: `_`, or `*` under `allowStarWildcardImport`.
    fn at_wildcard(&self) -> bool {
        self.at(C::Underscore) || self.at_soft(self.dialect.allow_star_wildcard_import, "*")
    }

    /// `importWildcardOrName()`.
    fn import_wildcard_or_name(&mut self) -> P<u32> {
        let start = self.curr_index();
        self.auto_end_pos(start, |p| {
            if p.at_wildcard() {
                p.next();
                return Ok(p.mk(K::ImporteeWildcard, Vec::new()));
            }
            match p.curr_c() {
                C::KwGiven => {
                    p.next();
                    if p.at(C::Ident) {
                        let tpe = p.typ()?;
                        Ok(p.mk(K::ImporteeGiven, vec![(F::Tpe, tpe)]))
                    } else {
                        Ok(p.mk(K::ImporteeGivenAll, Vec::new()))
                    }
                }
                C::Ident => {
                    let value = super::names::ident_value(p.curr_text());
                    p.next();
                    let name = p.mk_value(K::NameIndeterminate, Vec::new(), value);
                    let name = p.at_pos1(start, name);
                    Ok(p.mk(K::ImporteeName, vec![(F::Name, name)]))
                }
                _ => p.expected_c(C::Ident),
            }
        })
    }

    /// `importeeRename(from)`.
    fn importee_rename(&mut self, from: u32) -> P<u32> {
        let start = self.beg_index(from);
        self.auto_end_pos(start, |p| {
            let to = p.import_wildcard_or_name()?;
            match p.kind(to) {
                K::ImporteeName => {
                    let name = p.field(to, F::Name).unwrap();
                    Ok(p.mk(K::ImporteeRename, vec![(F::Name, from), (F::Rename, name)]))
                }
                K::ImporteeWildcard => Ok(p.mk(K::ImporteeUnimport, vec![(F::Name, from)])),
                _ => p.refused("unreachable".to_string()),
            }
        })
    }

    /// `importee()`.
    fn importee(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            let from = p.import_wildcard_or_name()?;
            let renames = p.at(C::RightArrow) || p.at_soft(p.dialect.allow_as_for_import_rename, "as");
            match p.kind(from) {
                K::ImporteeName if renames => {
                    p.next();
                    let name = p.field(from, F::Name).unwrap();
                    p.importee_rename(name)
                }
                K::ImporteeWildcard if renames && (p.peek(C::Underscore) || p.peek_soft(p.dialect.allow_star_wildcard_import, "*")) => {
                    p.next();
                    p.next();
                    Ok(from)
                }
                _ => Ok(from),
            }
        })
    }

    /// `nonLocalDefOrDcl()`.
    pub(super) fn non_local_def_or_dcl(&mut self) -> P<u32> {
        let mut mods = Vec::new();
        self.annots_buf(&mut mods, true, false, true)?;
        self.modifiers_buf(&mut mods, false, false)?;
        self.def_or_dcl_or_secondary_ctor(mods)
    }

    /// The start of what `mods` begin (`modsToPos`): the first one's, else the current token.
    fn mods_start(&self, mods: &[u32]) -> i32 {
        mods.first().map_or_else(|| self.curr_index(), |&m| self.beg_index(m))
    }

    fn mods_fields(mods: &[u32]) -> Vec<(F, u32)> {
        mods.iter().map(|&m| (F::Mods, m)).collect()
    }

    /// `defOrDclOrSecondaryCtor(mods)`.
    pub(super) fn def_or_dcl_or_secondary_ctor(&mut self, mods: Vec<u32>) -> P<u32> {
        let only_inline = mods.len() == 1 && self.kind(mods[0]) == K::ModInline;
        match self.curr_c() {
            C::KwVal | C::KwVar => self.pat_def_or_dcl(mods),
            C::KwGiven => self.given_decl(mods),
            C::KwDef => {
                let start = self.mods_start(&mods);
                let def_at = self.curr_offset();
                self.auto_end_pos(start, |p| {
                    p.next();
                    if !p.at(C::KwThis) {
                        p.fun_def_rest_after_kw_def(mods)
                    } else if p.ctx.template_owner.is_secondary_ctor_allowed() {
                        if p.peek(C::LeftParen) {
                            let name = p.name_this();
                            let paramss = p.term_param_clauses_on_paren()?;
                            p.secondary_ctor_rest(mods, name, paramss)
                        } else {
                            Err(Stop::Refused(p.s.tokens[p.peek_index().max(0) as usize].start, "secondary constructor needs explicit parameter list".to_string()))
                        }
                    } else {
                        Err(Stop::Refused(def_at, "Illegal secondary constructor".to_string()))
                    }
                })
            }
            C::KwType => self.type_def_or_dcl(mods),
            C::KwCase if self.dialect.allow_enums && self.peek(C::Ident) => {
                let start = self.mods_start(&mods);
                self.auto_end_pos(start, |p| {
                    p.next();
                    if p.peek(C::Comma) { p.enum_repeated_case_def(mods) } else { p.enum_single_case_def(mods) }
                })
            }
            C::KwIf if only_inline => self.if_clause(mods),
            _ if self.s.is_expr_intro_of(self.curr_c(), self.curr_index()) && only_inline => self.inline_match_clause(mods),
            _ if self.s.is_kw_extension(self.curr_index()) => self.extension_group_decl(mods),
            _ => self.tmpl_def(mods, false),
        }
    }

    /// `patDefOrDcl(mods)`.
    fn pat_def_or_dcl(&mut self, mods: Vec<u32>) -> P<u32> {
        let start = self.mods_start(&mods);
        self.auto_end_pos(start, |p| {
            let is_val = p.at(C::KwVal);
            p.next();
            let mut lhs = vec![p.pattern2(false)?];
            while p.accept_opt(C::Comma) {
                lhs.push(p.pattern2(false)?);
            }
            for pat in lhs.iter_mut() {
                if p.kind(*pat) == K::TermName {
                    let v = p.mk(K::PatVar, vec![(F::Name, *pat)]);
                    *pat = p.copy_pos(*pat, v);
                }
            }
            let tp = p.typed_opt()?;
            let mut fields = Self::mods_fields(&mods);
            fields.extend(lhs.into_iter().map(|x| (F::Pats, x)));
            if p.accept_opt(C::Equals) {
                fields.extend(tp.map(|t| (F::Decltpe, t)));
                let rhs = p.expr()?;
                Ok(if is_val {
                    fields.push((F::Rhs, rhs));
                    p.mk(K::DefnVal, fields)
                } else {
                    fields.push((F::Body, rhs));
                    p.mk(K::DefnVar, fields)
                })
            } else {
                let Some(tp) = tp else { return p.refused("declaration requires a type".to_string()) };
                fields.push((F::Decltpe, tp));
                Ok(p.mk(if is_val { K::DeclVal } else { K::DeclVar }, fields))
            }
        })
    }

    /// `givenDecl(mods)`.
    fn given_decl(&mut self, mods: Vec<u32>) -> P<u32> {
        let start = self.mods_start(&mods);
        self.auto_end_pos(start, |p| {
            p.accept(C::KwGiven)?;
            let sig = p.given_sig()?;
            let new_syntax_ok = p.dialect.allow_improved_type_classes_syntax;
            let used_new_syntax = new_syntax_ok && !p.prev(C::Colon);
            let init_pos = sig.decl_pos.unwrap_or_else(|| sig.decl_type.map_or_else(|| p.curr_index(), |t| p.beg_index(t)));
            let decltype = match sig.decl_type {
                Some(t) => t,
                None => match p.refinement(None)? {
                    Some(t) => t,
                    None => p.start_mod_type()?,
                },
            };
            let mut fields = Self::mods_fields(&mods);
            fields.push((F::Name, sig.name));
            fields.extend(sig.pcg.iter().map(|&g| (F::ParamClauseGroups, g)));
            match p.curr_c() {
                C::Equals => {
                    p.next();
                    fields.push((F::Decltpe, decltype));
                    fields.push((F::Body, p.expr()?));
                    Ok(p.mk(K::DefnGivenAlias, fields))
                }
                C::KwWith | C::LeftParen => {
                    let templ = p.within_owner(Owner::Given, |p| p.given_template(init_pos, decltype, used_new_syntax))?;
                    fields.push((F::Templ, templ));
                    Ok(p.mk(K::DefnGiven, fields))
                }
                C::Comma | C::LeftBrace | C::Colon if new_syntax_ok => {
                    let templ = p.within_owner(Owner::Given, |p| p.given_template(init_pos, decltype, used_new_syntax))?;
                    fields.push((F::Templ, templ));
                    Ok(p.mk(K::DefnGiven, fields))
                }
                _ => {
                    fields.push((F::Decltpe, decltype));
                    match p.kind(sig.name) {
                        K::TermName => Ok(p.mk(K::DeclGiven, fields)),
                        K::NameAnonymous if p.dialect.allow_improved_type_classes_syntax => Ok(p.mk(K::DeclGivenAnonymous, fields)),
                        _ => p.refused("abstract givens cannot be anonymous".to_string()),
                    }
                }
            }
        })
    }

    /// `givenDecl`'s `getDefnGiven()`: the template from the declared type's start.
    fn given_template(&mut self, init_pos: i32, decltype: u32, used_new_syntax: bool) -> P<u32> {
        let head_init = if !self.at(C::LeftParen) { self.init_impl(init_pos, decltype, Vec::new())? } else { self.init_rest_at(init_pos, decltype, true, false, false)? };
        let inits = self.template_parents_with_first(self.dialect.allow_improved_type_classes_syntax, true, head_init)?;
        let body = if self.accept_opt(C::KwWith) {
            if self.is_after_opt_new_line(C::LeftBrace) {
                self.template_body_on_left_brace()?
            } else if self.at(C::Indent) {
                self.auto_pos(|p| p.template_body_on_indent_raw())?
            } else {
                return self.refused("expected '{' or indentation".to_string());
            }
        } else if used_new_syntax {
            self.template_body_opt()?
        } else if inits.len() > 1 {
            let at = self.s.tokens[self.prev_index().max(0) as usize].start;
            return Err(Stop::Refused(at, "expected 'with' <body>".to_string()));
        } else {
            self.empty_template_body()
        };
        self.auto_end_pos(init_pos, |p| Ok(p.template_of(None, inits, body, Vec::new())))
    }

    /// `givenSig()`: tried as a named given's signature, else an anonymous one's name.
    fn given_sig(&mut self) -> P<GivenSig> {
        let sig = self.try_parse(|p| p.within_given_sig(|p| p.given_sig_tried()))?;
        Ok(match sig {
            Some(s) => s,
            None => GivenSig::named(self.anon_name()),
        })
    }

    fn within_given_sig<T>(&mut self, body: impl FnOnce(&mut Self) -> P<T>) -> P<T> {
        self.ctx.given_sig += 1;
        let out = body(self);
        self.ctx.given_sig -= 1;
        out
    }

    fn given_sig_tried(&mut self) -> P<Option<GivenSig>> {
        if !self.at(C::Ident) {
            return if self.is_after_opt_new_line(C::LeftBracket) {
                let name = self.anon_name();
                self.given_sig_on_bracket(name)
            } else if self.is_after_opt_new_line(C::LeftParen) {
                let name = self.anon_name();
                self.try_refusal(|p| p.given_sig_on_paren(name)).map(Option::flatten)
            } else {
                Ok(None)
            };
        }
        let ident_pos = self.curr_index();
        let value = super::names::ident_value(self.curr_text());
        let name = |p: &mut Self| {
            let n = p.mk_value(K::TermName, Vec::new(), value.clone());
            p.at_pos1(ident_pos, n)
        };
        let new_syntax = self.dialect.allow_improved_type_classes_syntax;
        if self.try_ahead(C::Colon) {
            if !self.next_if(!self.peek(C::Indent)) {
                Ok(None)
            } else if new_syntax {
                let n = name(self);
                self.given_sig_after_colon(n)
            } else {
                Ok(Some(GivenSig::named(name(self))))
            }
        } else if self.try_ahead(C::LeftBracket) {
            let n = name(self);
            self.given_sig_on_bracket(n)
        } else if self.try_ahead(C::LeftParen) {
            let n = name(self);
            self.given_sig_on_paren(n)
        } else if self.peek_c().eol() {
            self.next();
            if self.try_ahead(C::LeftBracket) {
                let n = name(self);
                self.given_sig_on_bracket(n)
            } else if self.try_ahead(C::LeftParen) {
                let n = name(self);
                self.given_sig_on_paren(n)
            } else {
                Ok(None)
            }
        } else if !new_syntax {
            Ok(None)
        } else if !self.try_ahead(C::RightArrow) {
            self.given_sig_other(|p| p.anon_name_at(ident_pos))
        } else {
            let anon = self.anon_name_at(ident_pos);
            let t = self.mk_value(K::TypeName, Vec::new(), value.clone());
            let t = self.at_pos1(ident_pos, t);
            self.given_sig_on_arrow(anon, t)
        }
    }

    /// `givenOldSyntaxColon()`.
    fn given_old_syntax_colon(&mut self) -> bool {
        self.at(C::Colon) && self.next_if(!self.peek(C::Indent))
    }

    /// `givenSigOnBracket(name)`: its group tried, a refusal no signature.
    fn given_sig_on_bracket(&mut self, name: u32) -> P<Option<GivenSig>> {
        let Some(pcg) = self.try_refusal(|p| p.member_param_clause_group_on_bracket())? else { return Ok(None) };
        if self.given_old_syntax_colon() {
            return Ok(Some(GivenSig { name, pcg: vec![pcg], decl_type: None, decl_pos: None }));
        }
        let tparams = self.field(pcg, F::TparamClause).unwrap();
        if self.nodes[pcg as usize].fields.iter().any(|&(f, _)| f == F::ParamClauses) {
            Ok(None)
        } else if self.kind(name) != K::NameAnonymous {
            let tn = self.name_as_type(name);
            let tpe_init = self.convert_name_type_param_clause_to_type(tn, tparams);
            let tpe = self.infix_type_rest(tpe_init, false, true)?;
            let anon = self.anon_name_at(self.beg_index(name));
            if self.dialect.allow_improved_type_classes_syntax && self.accept_opt(C::RightArrow) {
                let pc = self.convert_type_to_term_param_clause(tpe);
                self.given_sig_after_arrow_pc(anon, pc)
            } else {
                Ok(Some(GivenSig { name: anon, pcg: Vec::new(), decl_type: Some(tpe), decl_pos: None }))
            }
        } else if self.dialect.allow_improved_type_classes_syntax && self.accept_opt(C::RightArrow) {
            self.given_sig_after_arrow(name, tparams, Vec::new())
        } else {
            Ok(None)
        }
    }

    /// `givenSigOnParen(name)`.
    fn given_sig_on_paren(&mut self, name: u32) -> P<Option<GivenSig>> {
        let use_new_syntax = self.dialect.allow_improved_type_classes_syntax && self.kind(name) == K::NameAnonymous;
        if !use_new_syntax && !self.peek_soft(self.dialect.allow_given_using, "using") {
            return Ok(None);
        }
        let pcg = self.member_param_clause_group_on_paren()?;
        if self.given_old_syntax_colon() {
            return Ok(Some(GivenSig { name, pcg: vec![pcg], decl_type: None, decl_pos: None }));
        }
        let clauses: Vec<u32> = self.nodes[pcg as usize].fields.iter().filter(|&&(f, _)| f == F::ParamClauses).map(|&(_, c)| c).collect();
        if clauses.len() == 1 && use_new_syntax && self.accept_opt(C::RightArrow) {
            let tparams = self.field(pcg, F::TparamClause).unwrap();
            self.given_sig_after_arrow(name, tparams, clauses)
        } else {
            Ok(None)
        }
    }

    /// `givenSigAfterColon(name)`.
    fn given_sig_after_colon(&mut self, name: u32) -> P<Option<GivenSig>> {
        if self.is_after_opt_new_line(C::LeftParen) {
            let pc = self.term_param_clause_on_paren()?;
            if self.accept_opt(C::RightArrow) {
                self.given_sig_after_arrow_pc(name, pc)
            } else {
                let t = self.convert_term_param_clause_to_type(pc);
                Ok(Some(GivenSig { name, pcg: Vec::new(), decl_type: Some(t), decl_pos: None }))
            }
        } else if self.is_after_opt_new_line(C::LeftBracket) {
            let tpc = self.type_param_clause_on_bracket()?;
            self.accept(C::RightArrow)?;
            self.given_sig_after_arrow(name, tpc, Vec::new())
        } else {
            self.given_sig_other(move |_| name)
        }
    }

    /// `givenSigOther(name)`.
    fn given_sig_other(&mut self, name: impl FnOnce(&mut Self) -> u32) -> P<Option<GivenSig>> {
        let start = self.curr_index();
        let tpe = self.start_infix_type(true)?;
        if self.at(C::RightArrow) {
            let n = name(self);
            self.given_sig_on_arrow(n, tpe)
        } else {
            let n = name(self);
            Ok(Some(GivenSig { name: n, pcg: Vec::new(), decl_type: Some(tpe), decl_pos: Some(start) }))
        }
    }

    /// `givenSigOnArrow(name, tpe)`.
    fn given_sig_on_arrow(&mut self, name: u32, tpe: u32) -> P<Option<GivenSig>> {
        self.next();
        let pc = self.convert_type_to_term_param_clause(tpe);
        self.given_sig_after_arrow_pc(name, pc)
    }

    /// `givenSigAfterArrow(name, pc)`: an empty type clause where the term clause is.
    fn given_sig_after_arrow_pc(&mut self, name: u32, pc: u32) -> P<Option<GivenSig>> {
        let tpc = self.mk(K::TypeParamClause, Vec::new());
        let tpc = self.at_pos_empty(self.beg_index(pc), tpc);
        self.given_sig_after_arrow(name, tpc, vec![pc])
    }

    /// `givenSigAfterArrow(name, tpc, pcbuf)`: the conditions before the given's type, each group closed at its `=>`.
    fn given_sig_after_arrow(&mut self, name: u32, mut tpc: u32, mut pcbuf: Vec<u32>) -> P<Option<GivenSig>> {
        let mut pcgbuf: Vec<u32> = Vec::new();
        loop {
            let arrow_pos = self.prev_index();
            if pcbuf.last().is_some_and(|&pc| self.field(pc, F::Values).is_none()) {
                self.flush_pc_buf(tpc, arrow_pos, &mut pcbuf, &mut pcgbuf);
                return Ok(Some(GivenSig { name, pcg: pcgbuf, decl_type: None, decl_pos: None }));
            } else if self.is_after_opt_new_line(C::LeftParen) {
                let pc = self.term_param_clause_on_paren()?;
                if self.accept_opt(C::RightArrow) {
                    pcbuf.push(pc);
                } else {
                    self.flush_pc_buf(tpc, arrow_pos, &mut pcbuf, &mut pcgbuf);
                    let t = self.convert_term_param_clause_to_type(pc);
                    return Ok(Some(GivenSig { name, pcg: pcgbuf, decl_type: Some(t), decl_pos: None }));
                }
            } else if self.is_after_opt_new_line(C::LeftBracket) {
                self.flush_pc_buf(tpc, arrow_pos, &mut pcbuf, &mut pcgbuf);
                tpc = self.type_param_clause_on_bracket()?;
                self.accept(C::RightArrow)?;
            } else {
                let start = self.curr_index();
                let tpe = self.start_infix_type(true)?;
                if self.accept_opt(C::RightArrow) {
                    let pc = self.convert_type_to_term_param_clause(tpe);
                    pcbuf.push(pc);
                } else {
                    self.flush_pc_buf(tpc, arrow_pos, &mut pcbuf, &mut pcgbuf);
                    return Ok(Some(GivenSig { name, pcg: pcgbuf, decl_type: Some(tpe), decl_pos: Some(start) }));
                }
            }
        }
    }

    /// `givenSigAfterArrow`'s `flushPcBuf()`: the clauses read a group from the type clause to the last `=>`.
    fn flush_pc_buf(&mut self, tpc: u32, arrow_pos: i32, pcbuf: &mut Vec<u32>, pcgbuf: &mut Vec<u32>) {
        if !pcbuf.is_empty() || self.field(tpc, F::Values).is_some() {
            let start = self.beg_index(tpc);
            let g = self.param_clause_group(tpc, std::mem::take(pcbuf));
            pcgbuf.push(self.at_pos(start, arrow_pos, g));
        }
        pcbuf.clear();
    }

    /// `convertTypeParamToType(param)`.
    fn convert_type_param_to_type(&mut self, param: u32) -> u32 {
        let name = self.field(param, F::Name).unwrap();
        let tn = self.name_as_type(name);
        let tpc = self.field(param, F::TparamClause).unwrap();
        self.convert_name_type_param_clause_to_type(tn, tpc)
    }

    /// `convertNameTypeParamClauseToType(name, tpc)`: `A[B]` of a clause read as type parameters.
    fn convert_name_type_param_clause_to_type(&mut self, name: u32, tpc: u32) -> u32 {
        let params: Vec<u32> = self.children(tpc).collect();
        if params.is_empty() {
            return name;
        }
        let args: Vec<(F, u32)> = params.into_iter().map(|tp| (F::Values, self.convert_type_param_to_type(tp))).collect();
        let clause = self.mk(K::TypeArgClause, args);
        let clause = self.copy_pos(tpc, clause);
        let app = self.mk(K::TypeApply, vec![(F::Tpe, name), (F::ArgClause, clause)]);
        let (start, end) = (self.beg_index(name), self.end_index(tpc));
        self.at_pos(start, end, app)
    }

    /// `convertTermParamToType(param)`.
    fn convert_term_param_to_type(&mut self, param: u32) -> u32 {
        let name = self.field(param, F::Name).unwrap();
        let tn = self.name_as_type(name);
        let Some(tpe) = self.field(param, F::Decltpe) else { return tn };
        let mods: Vec<u32> = self.nodes[param as usize].fields.iter().filter(|&&(f, _)| f == F::Mods).map(|&(_, m)| m).collect();
        if mods.is_empty() && self.kind(name) == K::NameAnonymous {
            tpe
        } else {
            let mut fields = vec![(F::Name, tn), (F::Tpe, tpe)];
            fields.extend(mods.into_iter().map(|m| (F::Mods, m)));
            let t = self.mk(K::TypeTypedParam, fields);
            self.copy_pos(param, t)
        }
    }

    /// `convertTermParamClauseToType(pc)`.
    fn convert_term_param_clause_to_type(&mut self, pc: u32) -> u32 {
        let params: Vec<u32> = self.nodes[pc as usize].fields.iter().filter(|&&(f, _)| f == F::Values).map(|&(_, x)| x).collect();
        let types: Vec<u32> = params.into_iter().map(|x| self.convert_term_param_to_type(x)).collect();
        match types.as_slice() {
            [] => {
                let u = self.mk_value(K::LitUnit, Vec::new(), String::new());
                self.copy_pos(pc, u)
            }
            [v] if self.kind(*v) != K::TypeTypedParam => *v,
            _ => {
                let fields = types.into_iter().map(|t| (F::Args, t)).collect();
                let t = self.mk(K::TypeTuple, fields);
                self.copy_pos(pc, t)
            }
        }
    }

    /// `convertTypeToTermParamClause(tpe)`: a clause of one anonymous parameter of type `tpe`.
    fn convert_type_to_term_param_clause(&mut self, tpe: u32) -> u32 {
        let anon = self.anon_name_at(self.beg_index(tpe));
        let param = self.mk(K::Param, vec![(F::Name, anon), (F::Decltpe, tpe)]);
        let param = self.copy_pos(tpe, param);
        let pc = self.mk(K::ParamClause, vec![(F::Values, param)]);
        self.copy_pos(tpe, pc)
    }

    /// `extensionGroupDecl(mods)`.
    fn extension_group_decl(&mut self, mods: Vec<u32>) -> P<u32> {
        let start = self.mods_start(&mods);
        self.auto_end_pos(start, |p| {
            p.next();
            p.ctx.extension_sig += 1;
            let pcg = p.member_param_clause_group(true);
            p.ctx.extension_sig -= 1;
            let pcg = pcg?;
            p.new_lines_opt();
            let body = match p.curr_c() {
                C::LeftBrace => {
                    let start = p.curr_index();
                    p.next();
                    let stats = p.stat_seq(StatPf::Template)?;
                    p.in_braces_after_open(())?;
                    let block = p.mk(K::TermBlock, stats.into_iter().map(|s| (F::Stats, s)).collect());
                    let end = p.prev_index();
                    p.at_pos(start, end, block)
                }
                C::Indent => {
                    let start = p.curr_index();
                    p.next();
                    let stats = p.stat_seq(StatPf::Template)?;
                    let body = match stats.as_slice() {
                        [t] if !p.is_preceded_by_detached_comment(p.curr_index(), p.end_index(*t)) => *t,
                        _ => p.mk(K::TermBlock, stats.into_iter().map(|s| (F::Stats, s)).collect()),
                    };
                    p.indented_after_open(())?;
                    let end = p.prev_index();
                    p.at_pos_opt(start, end, body)
                }
                _ if p.s.is_def_intro(p.curr_index()) => p.non_local_def_or_dcl()?,
                _ => return p.refused("Extension without extension method".to_string()),
            };
            let mut fields = Vec::new();
            fields.extend(pcg.map(|g| (F::ParamClauseGroup, g)));
            fields.push((F::Body, body));
            Ok(p.mk(K::DefnExtensionGroup, fields))
        })
    }

    /// `funDefRestAfterKwDef(mods)`.
    fn fun_def_rest_after_kw_def(&mut self, mods: Vec<u32>) -> P<u32> {
        let name = self.term_name()?;
        let groups = if self.dialect.allow_param_clause_interleaving { self.member_param_clause_groups()? } else { self.member_param_clause_group(true)?.into_iter().collect() };
        self.ctx.return_type += 1;
        let restype = self.typed_opt();
        self.ctx.return_type -= 1;
        let restype = restype?;
        let mut fields = Self::mods_fields(&mods);
        fields.push((F::Name, name));
        fields.extend(groups.into_iter().map(|g| (F::ParamClauseGroups, g)));
        if self.accept_opt(C::Equals) {
            let macro_ = self.accept_opt(C::KwMacro);
            fields.extend(restype.map(|t| (F::Decltpe, t)));
            fields.push((F::Body, self.expr()?));
            Ok(self.mk(if macro_ { K::DefnMacro } else { K::DefnDef }, fields))
        } else if self.at_stat_seq_end() || self.at_stat_sep() {
            let t = match restype {
                Some(t) => t,
                None => self.procedure_syntax_decl_type(),
            };
            fields.push((F::Decltpe, t));
            Ok(self.mk(K::DeclDef, fields))
        } else if restype.is_none() && self.is_after_opt_new_line(C::LeftBrace) {
            let t = self.procedure_syntax_decl_type();
            fields.push((F::Decltpe, t));
            fields.push((F::Body, self.expr()?));
            Ok(self.mk(K::DefnDef, fields))
        } else {
            self.expected_c(C::Equals)
        }
    }

    /// `procedureSyntaxDeclType`: `Unit` of no width at the current token.
    fn procedure_syntax_decl_type(&mut self) -> u32 {
        let t = self.mk_value(K::TypeName, Vec::new(), "Unit".to_string());
        self.at_cur_pos_empty(t)
    }

    /// `typeDefOrDcl(mods)`.
    pub(super) fn type_def_or_dcl(&mut self, mods: Vec<u32>) -> P<u32> {
        let start = self.mods_start(&mods);
        self.auto_end_pos(start, |p| {
            p.accept(C::KwType)?;
            p.new_lines_opt();
            let name = p.type_name()?;
            let tparams = p.type_param_clause_opt()?;
            let bounds = p.type_bounds()?;
            let mut fields = Self::mods_fields(&mods);
            fields.extend([(F::Name, name), (F::TparamClause, tparams)]);
            if p.accept_opt(C::Equals) {
                fields.push((F::Body, p.type_indented_opt()?));
                fields.push((F::Bounds, bounds));
                Ok(p.mk(K::DefnType, fields))
            } else {
                fields.push((F::Bounds, bounds));
                Ok(p.mk(K::DeclType, fields))
            }
        })
    }

    /// `topLevelTmplDef`.
    pub(super) fn top_level_tmpl_def(&mut self) -> P<u32> {
        let mut mods = Vec::new();
        self.annots_buf(&mut mods, true, false, true)?;
        self.modifiers_buf(&mut mods, false, false)?;
        self.tmpl_def(mods, true)
    }

    /// `tmplDef(mods, okTopLevel)`.
    pub(super) fn tmpl_def(&mut self, mods: Vec<u32>, ok_top_level: bool) -> P<u32> {
        let start = self.mods_start(&mods);
        self.auto_end_pos_opt(start, |p| match p.curr_c() {
            C::KwTrait => p.trait_def(mods),
            C::KwEnum => p.enum_def(mods),
            C::KwClass => p.class_def(mods),
            C::KwObject => p.object_def(mods),
            C::At => p.refused("Annotations must precede keyword modifiers".to_string()),
            _ if ok_top_level && p.dialect.allow_toplevel_statements && p.s.is_def_intro(p.curr_index()) => p.def_or_dcl_or_secondary_ctor(mods),
            _ => p.refused("expected start of definition".to_string()),
        })
    }

    /// `traitDef(mods)`.
    fn trait_def(&mut self, mods: Vec<u32>) -> P<u32> {
        self.next();
        let name = self.type_name()?;
        self.within_owner(Owner::Trait, |p| {
            let tparams = p.type_param_clause_opt()?;
            let ctor = p.primary_ctor()?;
            let templ = p.template_opt()?;
            let mut fields = Self::mods_fields(&mods);
            fields.extend([(F::Name, name), (F::TparamClause, tparams), (F::Ctor, ctor), (F::Templ, templ)]);
            Ok(p.mk(K::DefnTrait, fields))
        })
    }

    /// `classDef(mods)`.
    fn class_def(&mut self, mods: Vec<u32>) -> P<u32> {
        self.next();
        let name = self.type_name()?;
        let tparams = self.type_param_clause_opt()?;
        self.within_owner(Owner::Class, |p| {
            let ctor = p.primary_ctor()?;
            let templ = p.template_opt()?;
            let mut fields = Self::mods_fields(&mods);
            fields.extend([(F::Name, name), (F::TparamClause, tparams), (F::Ctor, ctor), (F::Templ, templ)]);
            Ok(p.mk(K::DefnClass, fields))
        })
    }

    /// `enumDef(mods)`.
    fn enum_def(&mut self, mods: Vec<u32>) -> P<u32> {
        self.within_owner(Owner::Enum, |p| {
            p.next();
            let name = p.type_name()?;
            let tparams = p.type_param_clause_opt()?;
            let ctor = p.primary_ctor()?;
            let templ = p.template_opt()?;
            let mut fields = Self::mods_fields(&mods);
            fields.extend([(F::Name, name), (F::TparamClause, tparams), (F::Ctor, ctor), (F::Templ, templ)]);
            Ok(p.mk(K::DefnEnum, fields))
        })
    }

    /// `enumRepeatedCaseDef(mods)`.
    fn enum_repeated_case_def(&mut self, mods: Vec<u32>) -> P<u32> {
        let mut fields = Self::mods_fields(&mods);
        fields.push((F::Cases, self.term_name()?));
        while self.accept_opt(C::Comma) {
            fields.push((F::Cases, self.term_name()?));
        }
        Ok(self.mk(K::DefnRepeatedEnumCase, fields))
    }

    /// `enumSingleCaseDef(mods)`.
    fn enum_single_case_def(&mut self, mods: Vec<u32>) -> P<u32> {
        self.within_owner(Owner::Enum, |p| {
            let name = p.term_name()?;
            let tparams = p.type_param_clause_opt()?;
            let ctor = p.primary_ctor()?;
            let inits = if p.accept_opt(C::KwExtends) { p.template_parents(true)? } else { Vec::new() };
            let mut fields = Self::mods_fields(&mods);
            fields.extend([(F::Name, name), (F::TparamClause, tparams), (F::Ctor, ctor)]);
            fields.extend(inits.into_iter().map(|i| (F::Inits, i)));
            Ok(p.mk(K::DefnEnumCase, fields))
        })
    }

    /// `objectDef(mods)`.
    fn object_def(&mut self, mods: Vec<u32>) -> P<u32> {
        self.next();
        let name = self.term_name()?;
        self.within_owner(Owner::Object, |p| {
            let templ = p.template_opt()?;
            let mut fields = Self::mods_fields(&mods);
            fields.extend([(F::Name, name), (F::Templ, templ)]);
            Ok(p.mk(K::DefnObject, fields))
        })
    }

    /* -------- CONSTRUCTORS -------- */

    /// `primaryCtor()`.
    fn primary_ctor(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            let no_ctor = !p.ctx.template_owner.is_primary_ctor_allowed(&p.dialect);
            let mut mods = Vec::new();
            if !no_ctor {
                p.annots_buf(&mut mods, false, true, false)?;
                p.ctor_modifiers(&mut mods)?;
            }
            let name = p.anon_name();
            let paramss = if no_ctor { Vec::new() } else { p.term_param_clauses()? };
            let mut fields = Self::mods_fields(&mods);
            fields.push((F::Name, name));
            fields.extend(paramss.into_iter().map(|c| (F::ParamClauses, c)));
            Ok(p.mk(K::CtorPrimary, fields))
        })
    }

    /// `secondaryCtorRest(mods, name, paramss)`.
    fn secondary_ctor_rest(&mut self, mods: Vec<u32>, name: u32, paramss: Vec<u32>) -> P<u32> {
        let has_left_brace = self.is_after_opt_new_line(C::LeftBrace) || {
            self.accept(C::Equals)?;
            self.at(C::LeftBrace)
        };
        let body = self.auto_pos(|p| {
            if has_left_brace {
                p.next();
                let b = p.constr_internal()?;
                p.in_braces_after_open(b)
            } else if p.accept_opt(C::Indent) {
                let b = p.constr_internal()?;
                p.indented_after_open(b)
            } else {
                let init = p.init_inside_constructor()?;
                Ok(p.mk(K::CtorBlock, vec![(F::Init, init)]))
            }
        })?;
        let mut fields = Self::mods_fields(&mods);
        fields.push((F::Name, name));
        fields.extend(paramss.into_iter().map(|c| (F::ParamClauses, c)));
        fields.push((F::Body, body));
        Ok(self.mk(K::CtorSecondary, fields))
    }

    /// `constrInternal()`.
    fn constr_internal(&mut self) -> P<u32> {
        let init = self.init_inside_constructor()?;
        let stats = if self.accept_if_stat_sep() { self.block_stat_seq(false)? } else { Vec::new() };
        let mut fields = vec![(F::Init, init)];
        fields.extend(stats.into_iter().map(|s| (F::Stats, s)));
        Ok(self.mk(K::CtorBlock, fields))
    }

    /// `initInsideConstructor()`: `this(...)`, its type `this.type`'s.
    fn init_inside_constructor(&mut self) -> P<u32> {
        let start = self.curr_index();
        self.expect(C::KwThis)?;
        let t = self.anon_this();
        let tpe = self.mk(K::TypeSingleton, vec![(F::Ref, t)]);
        let tpe = self.copy_pos(t, tpe);
        self.init_rest_at(start, tpe, true, false, true)
    }

    /// `initInsideTemplate()`.
    fn init_inside_template(&mut self) -> P<u32> {
        let start = self.curr_index();
        let tpe = self.start_mod_type()?;
        self.init_rest_at(start, tpe, true, false, false)
    }

    /// `initImpl(startPos, tpe)(argss)`: an `Init` of an anonymous name after its type.
    fn init_impl(&mut self, start: i32, tpe: u32, argss: Vec<u32>) -> P<u32> {
        self.auto_end_pos(start, |p| {
            let name = p.anon_name();
            let mut fields = vec![(F::Tpe, tpe), (F::Name, name)];
            fields.extend(argss.into_iter().map(|a| (F::ArgClauses, a)));
            Ok(p.mk(K::Init, fields))
        })
    }

    /// `initRestAt(startPos, tpe)(allowArgss, insidePrimaryCtorAnnot, allowBraces)`.
    pub(super) fn init_rest_at(&mut self, start: i32, tpe: u32, allow_argss: bool, inside_primary_ctor_annot: bool, allow_braces: bool) -> P<u32> {
        self.auto_end_pos(start, |p| {
            let name = p.anon_name();
            let mut argss = Vec::new();
            if p.init_arg_clause(&mut argss, inside_primary_ctor_annot, allow_braces)? && allow_argss {
                while p.init_arg_clause(&mut argss, inside_primary_ctor_annot, allow_braces)? {}
            }
            let mut fields = vec![(F::Tpe, tpe), (F::Name, name)];
            fields.extend(argss.into_iter().map(|a| (F::ArgClauses, a)));
            Ok(p.mk(K::Init, fields))
        })
    }

    /// `initRestAt`'s `maybeBody()`: an argument clause, past a line break where the next line indents it.
    fn init_arg_clause(&mut self, argss: &mut Vec<u32>, inside_primary_ctor_annot: bool, allow_braces: bool) -> P<bool> {
        if self.at_eol() {
            let ok = match self.peek_c() {
                C::LeftParen => self.is_indenting_or_eol(false),
                C::LeftBrace => self.is_indenting_or_eol(allow_braces),
                _ => false,
            };
            self.next_if(ok);
        }
        let allow_parens = !inside_primary_ctor_annot
            || match self.peek_c() {
                C::RightParen => argss.is_empty(),
                C::Ident => !self.ahead(|p| p.peek(C::Colon)),
                C::At => false,
                _ => !self.s.is_modifier(self.peek_index()),
            };
        match self.curr_c() {
            C::LeftParen if allow_parens => {
                argss.push(self.get_arg_clause_on_paren(Location::NoStat)?);
                Ok(true)
            }
            C::LeftBrace if allow_braces => {
                argss.push(self.get_arg_clause_on_brace()?);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /* -------- SELFS -------- */

    /// `selfEither()`: the self of a template's body, none where `=>` does not follow it.
    pub(super) fn self_either(&mut self) -> P<Option<u32>> {
        let start = self.curr_index();
        let name = match self.curr_c() {
            C::Ident => self.term_name()?,
            C::KwThis => self.name_this(),
            C::Underscore => self.name_placeholder(),
            _ => return Ok(None),
        };
        let decltpe = if !(self.peek_c().indentation() || self.peek_c().eol()) {
            if self.accept_opt(C::Colon) { Some(self.start_infix_type(false)?) } else { None }
        } else if !self.at(C::Colon) {
            None
        } else {
            return Ok(None);
        };
        if !self.accept_opt(C::RightArrow) {
            return Ok(None);
        }
        self.auto_end_pos(start, |p| {
            let mut fields = vec![(F::Name, name)];
            fields.extend(decltpe.map(|t| (F::Decltpe, t)));
            Ok(p.mk(K::SelfNode, fields))
        })
        .map(Some)
    }

    /* -------- TEMPLATES -------- */

    /// `templateParentsWithFirst(allowComma, allowWithBody)(first)`.
    fn template_parents_with_first(&mut self, allow_comma: bool, allow_with_body: bool, first: u32) -> P<Vec<u32>> {
        let with_sep = |p: &Self| p.at(C::KwWith) && !(allow_with_body && (p.peek_c().whitespace() || p.peek(C::LeftBrace)));
        let by_with = with_sep(self);
        let by_comma = !by_with && allow_comma && self.dialect.allow_comma_separated_extend && self.at(C::Comma);
        let mut inits = vec![first];
        if by_with || by_comma {
            loop {
                self.next();
                inits.push(self.init_inside_template()?);
                if !(if by_with { with_sep(self) } else { self.at(C::Comma) }) {
                    break;
                }
            }
        }
        Ok(inits)
    }

    /// `templateParents(afterExtend)`.
    fn template_parents(&mut self, after_extend: bool) -> P<Vec<u32>> {
        let first = self.init_inside_template()?;
        self.template_parents_with_first(after_extend, false, first)
    }

    /// `derivesClasses()`.
    fn derives_classes(&mut self) -> P<Vec<u32>> {
        let enabled = self.dialect.allow_derives;
        let at_derives = if self.curr_c().eol() {
            let ok = self.peek_soft(enabled, "derives");
            self.next_if(ok)
        } else {
            self.at_soft(enabled, "derives")
        };
        let mut out = Vec::new();
        if at_derives {
            self.next();
            self.new_line_opt();
            loop {
                out.push(self.start_mod_type()?);
                if !self.accept_opt(C::Comma) {
                    break;
                }
            }
        }
        Ok(out)
    }

    /// A `Template` of its fields in scalameta's order.
    fn template_of(&mut self, early: Option<u32>, inits: Vec<u32>, body: u32, derived: Vec<u32>) -> u32 {
        let mut fields = Vec::new();
        fields.extend(early.map(|e| (F::EarlyClause, e)));
        fields.extend(inits.into_iter().map(|i| (F::Inits, i)));
        fields.push((F::Body, body));
        fields.extend(derived.into_iter().map(|d| (F::Derives, d)));
        self.mk(K::Template, fields)
    }

    /// `templateAfterExtends(parents, edefs)`.
    fn template_after_extends(&mut self, parents: Vec<u32>, edefs: Option<u32>) -> P<u32> {
        let derived = self.derives_classes()?;
        let body = self.template_body_opt()?;
        Ok(self.template_of(edefs, parents, body, derived))
    }

    /// `template(afterExtend)`.
    pub(super) fn template(&mut self, after_extend: bool) -> P<u32> {
        self.auto_pos(|p| {
            if p.is_after_opt_new_line(C::LeftBrace) {
                let body = p.template_body_on_left_brace()?;
                if p.at(C::KwWith) && p.field(body, F::SelfDecl).is_none() {
                    let edefs: Vec<(F, u32)> = p.nodes[body as usize].fields.iter().map(|&(_, s)| (F::Stats, s)).collect();
                    p.next();
                    let parents = p.template_parents(after_extend)?;
                    let early = p.mk(K::StatBlock, edefs);
                    let early = p.copy_pos(body, early);
                    p.template_after_extends(parents, Some(early))
                } else {
                    Ok(p.template_of(None, Vec::new(), body, Vec::new()))
                }
            } else {
                let parents = if p.at(C::Colon) { Vec::new() } else { p.template_parents(after_extend)? };
                p.template_after_extends(parents, None)
            }
        })
    }

    /// `templateOpt()`.
    pub(super) fn template_opt(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            let on_extends = p.accept_opt(C::KwExtends) || p.ctx.template_owner == Owner::Trait && p.accept_opt(C::Subtype);
            if on_extends { p.template(true) } else { p.template_after_extends(Vec::new(), None) }
        })
    }

    /// `emptyTemplateBody()`.
    fn empty_template_body(&mut self) -> u32 {
        let b = self.mk(K::TemplateBody, Vec::new());
        self.at_cur_pos_empty(b)
    }

    /// `templateBodyOnLeftBrace()`.
    fn template_body_on_left_brace(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            p.next();
            let b = p.template_stat_seq()?;
            p.in_braces_after_open(b)
        })
    }

    /// `templateBodyOnIndentRaw()`.
    fn template_body_on_indent_raw(&mut self) -> P<u32> {
        self.next();
        let b = self.template_stat_seq()?;
        self.indented_after_open(b)
    }

    /// `templateBodyOpt()`.
    fn template_body_opt(&mut self) -> P<u32> {
        if self.is_after_opt_new_line(C::LeftBrace) {
            self.template_body_on_left_brace()
        } else if self.at(C::Colon) && (self.peek_c().indentation() || self.peek_c().eol()) {
            self.auto_pos(|p| {
                p.next();
                if !p.at(C::Indent) {
                    return p.refused(format!("template body: `indent` expected but `{}` found", super::stream::found_name(p.curr_c(), p.curr_text())));
                }
                p.template_body_on_indent_raw()
            })
        } else if self.at(C::LeftParen) {
            let owner = self.ctx.template_owner;
            if owner.is_primary_ctor_allowed(&self.dialect) {
                self.refused("unexpected opening parenthesis".to_string())
            } else {
                self.refused(format!("{} may not have parameters", owner.what()))
            }
        } else {
            Ok(self.empty_template_body())
        }
    }

    /// The node `n` holds in its field `f`, its first.
    pub(super) fn field(&self, n: u32, f: F) -> Option<u32> {
        self.nodes[n as usize].fields.iter().find(|&&(g, _)| g == f).map(|&(_, c)| c)
    }

    /// `isPrecededByDetachedComment(idx, end)`: a comment on its own line between `end` and `idx`.
    pub(super) fn is_preceded_by_detached_comment(&self, idx: i32, end: i32) -> bool {
        let mut idx = idx;
        while idx > end {
            match self.s.tokens[idx as usize].c {
                C::Comment => return self.s.is_preceded_by_nl(idx),
                c if c.whitespace() => idx -= 1,
                _ => return false,
            }
        }
        false
    }

    /// `Try(body)`: a refusal no result, any other stop the parse's.
    pub(super) fn try_refusal<T>(&mut self, body: impl FnOnce(&mut Self) -> P<T>) -> P<Option<T>> {
        let saved = self.save();
        match body(self) {
            Ok(t) => Ok(Some(t)),
            Err(Stop::Refused(..)) => {
                self.restore(saved);
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}
