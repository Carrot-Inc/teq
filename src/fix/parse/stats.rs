//! ScalametaParser.scala's "statseqs" section: the source and its packages, the statement sequences of templates,
//! blocks and packages, and the statements each admits (`topStat`, `consumeStat`, `templateStat`, `blockStatSeq`).

use super::{Owner, Parser, P};
use crate::fix::nodes::{F, K};
use crate::fix::scanner::C;

/// A statement sequence's partial function over the current token (`statpf`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum StatPf {
    /// `topStat`: a file's statements.
    Top,
    /// `consumeStat`: a script's, which admits terms.
    Consume,
    /// `templateStat`.
    Template,
}

/// `Location`: where an expression stands, which decides a placeholder function and a parameter's type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Location {
    NoStat,
    BlockStat,
    TemplateStat,
    PostfixStat,
    UnquoteStat,
}

impl Location {
    pub(super) fn anon_func_ok(self) -> bool {
        !matches!(self, Location::PostfixStat | Location::UnquoteStat)
    }
    pub(super) fn func_param_ok(self) -> bool {
        matches!(self, Location::NoStat | Location::BlockStat)
    }
    pub(super) fn full_type_ok(self) -> bool {
        matches!(self, Location::NoStat | Location::PostfixStat)
    }
}

impl<'a> Parser<'a> {
    /// `entrypointSource`, `source()`.
    pub(super) fn entrypoint_source(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            p.accept_opt(C::Shebang);
            let statpf = if p.dialect.allow_toplevel_terms { StatPf::Consume } else { StatPf::Top };
            let mut wrappers: Vec<(i32, u32, i32)> = Vec::new();
            loop {
                p.skip_all_stat_sep();
                if p.at(C::KwPackage) && !p.peek(C::KwObject) {
                    p.next();
                    let start = p.prev_index();
                    let qid = p.qual_id()?;
                    if p.next_if_colon_indent() {
                        let delim = p.prev_index();
                        let stats = p.in_package_on_open(start, qid, delim, C::Outdent, statpf)?;
                        return Ok(p.wrap_packages(&wrappers, stats));
                    } else if p.is_after_opt_new_line(C::LeftBrace) {
                        let delim = p.curr_index();
                        let stats = p.in_package_on_open(start, qid, delim, C::RightBrace, statpf)?;
                        return Ok(p.wrap_packages(&wrappers, stats));
                    } else {
                        let delim = p.curr_index();
                        wrappers.push((start, qid, delim));
                    }
                } else {
                    let stats = p.stat_seq(statpf)?;
                    return Ok(p.wrap_packages(&wrappers, stats));
                }
            }
        })
    }

    /// `bracelessPackageStats`' continuations: the statements in the innermost braceless package's body, each package
    /// a statement of the one around it, the outermost the source's.
    fn wrap_packages(&mut self, wrappers: &[(i32, u32, i32)], mut stats: Vec<u32>) -> u32 {
        for &(start, qid, delim) in wrappers.iter().rev() {
            let pkg = self.get_package(start, qid, delim, stats);
            stats = vec![pkg];
        }
        let fields = stats.into_iter().map(|s| (F::Stats, s)).collect();
        self.mk(K::Source, fields)
    }

    /// `getPackage(pkgDelimPos)(stats)`: a `Pkg` from `start` over `qid` and a body from `delim`, both to the last
    /// token read.
    fn get_package(&mut self, start: i32, qid: u32, delim: i32, stats: Vec<u32>) -> u32 {
        let body = self.to_pkg_body(delim, stats);
        let pkg = self.mk(K::Pkg, vec![(F::Ref, qid), (F::Body, body)]);
        let end = self.prev_index();
        self.at_pos(start, end, pkg)
    }

    /// `toPkgBody(startPos)(stats)`.
    fn to_pkg_body(&mut self, start: i32, stats: Vec<u32>) -> u32 {
        let fields = stats.into_iter().map(|s| (F::Stats, s)).collect();
        let body = self.mk(K::PkgBody, fields);
        let end = self.prev_index();
        self.at_pos(start, end, body)
    }

    /// `inPackageOnOpen[T](pkgDelimPos)`: a package's body in braces or indented, the statements after it the
    /// source's.
    fn in_package_on_open(&mut self, start: i32, qid: u32, delim: i32, close: C, statpf: StatPf) -> P<Vec<u32>> {
        self.next();
        let mut inner = Vec::new();
        self.stat_seq_buf(&mut inner, statpf, "illegal start of definition")?;
        self.accept_after_opt_nl(close)?;
        let pkg = self.get_package(start, qid, delim, inner);
        let mut stats = vec![pkg];
        if !self.at(C::Eof) {
            self.accept_extended_stat_sep()?;
            self.stat_seq_buf(&mut stats, statpf, "illegal start of definition")?;
        }
        Ok(stats)
    }

    /// `nextIfColonIndent`: past a `:` an `Indent` follows.
    pub(super) fn next_if_colon_indent(&mut self) -> bool {
        self.at(C::Colon) && self.try_ahead(C::Indent)
    }

    /// `statSeq(statpf)`.
    pub(super) fn stat_seq(&mut self, statpf: StatPf) -> P<Vec<u32>> {
        let mut stats = Vec::new();
        self.stat_seq_buf(&mut stats, statpf, "illegal start of definition")?;
        Ok(stats)
    }

    /// `statSeqBuf(stats, statpf, errorMsg)`: whether it read an indented region.
    pub(super) fn stat_seq_buf(&mut self, stats: &mut Vec<u32>, statpf: StatPf, error: &str) -> P<bool> {
        let indented = self.accept_opt(C::Indent);
        while !self.at_stat_seq_end() {
            match self.stat_of(statpf)? {
                Some(s) => {
                    stats.push(s);
                    self.accept_extended_stat_sep_opt()?;
                }
                None => {
                    if !self.accept_if_stat_sep() {
                        let name = super::stream::found_name(self.curr_c(), self.curr_text());
                        return self.refused(format!("{} `{}`", error, name));
                    }
                }
            }
        }
        if indented {
            self.accept(C::Outdent)?;
        }
        Ok(indented)
    }

    /// The statement `statpf` reads at the current token, none where it is not defined there.
    fn stat_of(&mut self, statpf: StatPf) -> P<Option<u32>> {
        let index = self.curr_index();
        Ok(Some(match statpf {
            StatPf::Top => match self.curr_c() {
                C::KwPackage => self.package_or_package_object_def(StatPf::Top)?,
                C::KwImport => self.import_stmt()?,
                C::KwExport => self.export_stmt()?,
                _ if self.s.is_template_intro(index) => self.top_level_tmpl_def()?,
                _ if self.is_at_end_marker() => self.end_marker()?,
                _ if self.dialect.allow_toplevel_statements && self.s.is_def_intro(index) => self.non_local_def_or_dcl()?,
                _ => return Ok(None),
            },
            StatPf::Consume => match self.curr_c() {
                C::KwImport => self.import_stmt()?,
                C::KwExport => self.export_stmt()?,
                C::KwPackage => self.package_or_package_object_def(if self.dialect.allow_toplevel_terms { StatPf::Consume } else { StatPf::Top })?,
                _ if self.s.is_def_intro(index) => self.within_owner(Owner::Class, |p| p.non_local_def_or_dcl())?,
                _ if self.is_at_end_marker() => self.end_marker()?,
                _ if self.is_ident_or_expr_intro() => self.expr_at(Location::NoStat, true)?,
                _ => return Ok(None),
            },
            StatPf::Template => match self.curr_c() {
                C::KwImport => self.import_stmt()?,
                C::KwExport => self.export_stmt()?,
                _ if self.s.is_def_intro(index) => self.non_local_def_or_dcl()?,
                _ if self.is_at_end_marker() => self.end_marker()?,
                _ if self.is_ident_or_expr_intro() => self.expr_at(Location::TemplateStat, false)?,
                _ => return Ok(None),
            },
        }))
    }

    /// `isIdentOrExprIntro(currToken)`.
    pub(super) fn is_ident_or_expr_intro(&self) -> bool {
        self.s.is_ident_or_expr_intro(self.curr_c(), self.curr_index())
    }

    /// `packageOrPackageObjectDef(statpf)`.
    fn package_or_package_object_def(&mut self, statpf: StatPf) -> P<u32> {
        self.auto_pos(|p| {
            p.next();
            if p.accept_opt(C::KwObject) {
                p.within_owner(Owner::Object, |p| {
                    let name = p.term_name()?;
                    let templ = p.template_opt()?;
                    Ok(p.mk(K::PkgObject, vec![(F::Name, name), (F::Templ, templ)]))
                })
            } else {
                let qid = p.qual_id()?;
                let start = p.curr_index();
                let stats = if p.next_if_colon_indent() {
                    p.next();
                    let stats = p.stat_seq(statpf)?;
                    p.indented_after_open(stats)?
                } else {
                    p.new_line_opt();
                    if p.accept_opt(C::LeftBrace) {
                        let stats = p.stat_seq(statpf)?;
                        p.in_braces_after_open(stats)?
                    } else {
                        Vec::new()
                    }
                };
                let body = p.to_pkg_body(start, stats);
                Ok(p.mk(K::Pkg, vec![(F::Ref, qid), (F::Body, body)]))
            }
        })
    }

    /// `templateStatSeq()`: a template's body, its self first where one is written.
    pub(super) fn template_stat_seq(&mut self) -> P<u32> {
        let self_tree = self.try_parse(|p| p.self_either())?;
        let mut stats = Vec::new();
        let was_indented = self.stat_seq_buf(&mut stats, StatPf::Template, "illegal start of definition")?;
        if was_indented && self_tree.is_some() {
            self.stat_seq_buf(&mut stats, StatPf::Template, "illegal start of definition")?;
        }
        let mut fields = Vec::new();
        if let Some(s) = self_tree {
            fields.push((F::SelfDecl, s));
        }
        fields.extend(stats.into_iter().map(|s| (F::Stats, s)));
        Ok(self.mk(K::TemplateBody, fields))
    }

    /// `localDef(implicitMod)`.
    fn local_def(&mut self, implicit_mod: Option<u32>) -> P<u32> {
        let mut mods = Vec::new();
        self.annots_buf(&mut mods, true, false, true)?;
        if let Some(m) = implicit_mod {
            mods.push(m);
        }
        self.modifiers_buf(&mut mods, true, false)?;
        let plain = mods.iter().all(|&m| matches!(self.kind(m), K::ModImplicit | K::ModLazy | K::ModInline | K::ModInfix | K::ModAnnot));
        if plain { self.def_or_dcl_or_secondary_ctor(mods) } else { self.tmpl_def(mods, true) }
    }

    /// `blockStatSeq(allowRepeated)`: a block's statements, their separators read with them.
    pub(super) fn block_stat_seq(&mut self, allow_repeated: bool) -> P<Vec<u32>> {
        let mut stats = Vec::new();
        loop {
            self.skip_all_stat_sep();
            let index = self.curr_index();
            let stat = match self.curr_c() {
                C::Outdent => None,
                C::KwCase if self.is_case_intro_on_kw_case() => None,
                C::KwExport => Some(self.export_stmt()?),
                C::KwImport => Some(self.import_stmt()?),
                C::KwImplicit => {
                    let implicit_pos = self.curr_index();
                    self.next();
                    if self.at(C::Ident) && !self.s.is_soft_modifier(self.curr_index()) {
                        stats.push(self.implicit_closure(Location::BlockStat)?);
                        if !self.accept_if_extended_stat_sep() {
                            break;
                        }
                        continue;
                    }
                    let m = self.mk(K::ModImplicit, Vec::new());
                    let m = self.at_pos1(implicit_pos, m);
                    Some(self.local_def(Some(m))?)
                }
                c if !self.is_nonlocal_modifier(c) && self.s.is_def_intro(index) => Some(self.local_def(None)?),
                _ if self.is_at_end_marker() => Some(self.end_marker()?),
                _ if self.is_ident_or_expr_intro() => Some(self.expr_at(Location::BlockStat, allow_repeated)?),
                c if !self.s.might_start_stat(c, false) => None,
                _ => return self.refused("illegal start of statement".to_string()),
            };
            match stat {
                Some(s) => {
                    stats.push(s);
                    if !self.accept_if_extended_stat_sep() {
                        break;
                    }
                }
                None => break,
            }
        }
        Ok(stats)
    }

    /// `endMarker()`: `end` and the name or keyword after it.
    pub(super) fn end_marker(&mut self) -> P<u32> {
        self.auto_pos(|p| {
            p.next();
            let value = match p.curr_c() {
                C::Ident => super::names::ident_value(p.curr_text()),
                _ => p.curr_text().to_string(),
            };
            let name = p.term_name_value(value);
            Ok(p.mk(K::TermEndMarker, vec![(F::Name, name)]))
        })
    }

    /// `isNonlocalModifier(token)`: `private`, `protected`, `override`, `open`.
    fn is_nonlocal_modifier(&self, c: C) -> bool {
        matches!(c, C::KwPrivate | C::KwProtected | C::KwOverride) || self.at_soft(self.dialect.allow_open_class, "open")
    }

    /// `TemplateOwnerContext.within(owner)(body)`.
    pub(super) fn within_owner<T>(&mut self, owner: Owner, body: impl FnOnce(&mut Self) -> P<T>) -> P<T> {
        let old = self.ctx.template_owner;
        self.ctx.template_owner = owner;
        let out = body(self);
        self.ctx.template_owner = old;
        out
    }
}
