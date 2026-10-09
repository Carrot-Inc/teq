//! What the dialect flags (`src/dialect.rs`) reject in the program's own files: the
//! definition-level checks here, the checks at a call or a search where those happen.

use super::Worker;
use crate::ast::{mods, DefId, DefKind};
use crate::dialect::{self, rejected};
use crate::names;
use crate::source::{FileId, Span};
use crate::types::SymId;

impl<'a> Worker<'a> {
    /// Whether a construct at the current position is subject to the dialect: the flags are
    /// set and the file is the program's.
    #[inline]
    pub fn restricted(&self) -> bool {
        self.dialect.any() && !self.source(self.env.file).is_std
    }

    /// The dialect's checks on a top-level definition or a class member, before it is typed.
    pub fn restrict_def(&mut self, file: FileId, id: DefId) {
        if !self.dialect.any() || self.source(file).is_std {
            return;
        }
        let def = self.ast(file).def(id);
        let name = self.name_str(def.name);
        let dialect = self.dialect;
        if dialect.no_scala2_implicits {
            let implicit_clause = matches!(&def.kind, DefKind::Fun(f) if f.clauses.iter().any(|c| c.is_implicit));
            if def.mods & mods::IMPLICIT != 0 {
                let what = format!("the implicit definition `{}`", name);
                self.diags.error(file, def.span, rejected(&what, dialect::NO_SCALA2_IMPLICITS) + "; write it as a given");
            } else if implicit_clause {
                let what = format!("the implicit parameter clause of `{}`", name);
                self.diags.error(file, def.span, rejected(&what, dialect::NO_SCALA2_IMPLICITS) + "; write it as a using clause");
            }
        }
        if dialect.explicit_result_types && def.mods & mods::PRIVATE == 0 {
            let missing = match &def.kind {
                DefKind::Val { pat: None, ty: None, rhs: Some(_) } => Some("type"),
                DefKind::Fun(f) if f.ret.is_none() && f.body.is_some() && def.name != names::INIT => Some("result type"),
                _ => None,
            };
            if let Some(what) = missing {
                let msg = format!(
                    "`{}` needs a declared {} under the dialect flag `{}`: {}",
                    name,
                    what,
                    dialect::EXPLICIT_RESULT_TYPES,
                    dialect::cost(dialect::EXPLICIT_RESULT_TYPES)
                );
                self.diags.error(file, def.span, msg);
            }
        }
        if dialect.no_implicit_conversions && matches!(def.kind, DefKind::Fun(_) | DefKind::Given(_) | DefKind::Val { .. }) {
            if let Some(&sym) = self.def_syms.get(file.0 as usize, &id) {
                if self.is_conversion_def(sym) || self.is_conversion_given(sym) {
                    let what = format!("the implicit conversion `{}`", name);
                    let span = self.syms.sym(sym).span;
                    self.diags.error(file, span, rejected(&what, dialect::NO_IMPLICIT_CONVERSIONS) + "; write an extension method");
                }
            }
        }
    }

    /// A second alternative of a name, at its definition.
    pub fn restrict_overload(&mut self, sym: SymId) {
        let info = self.syms.sym(sym);
        let (file, span, name) = (info.file, info.span, info.name);
        if !self.dialect.no_overloading || self.source(file).is_std {
            return;
        }
        let what = format!("a second alternative of `{}`", self.name_str(name));
        self.diags.error(file, span, rejected(&what, dialect::NO_OVERLOADING) + "; give the alternatives distinct names");
    }

    /// A call of an inline method from the program.
    pub fn restrict_inline_call(&mut self, callee: SymId, span: Span) {
        if !self.dialect.no_inline || !self.restricted() {
            return;
        }
        let what = format!("the call of the inline method `{}`", self.name_str(self.syms.sym(callee).name));
        self.error(span, rejected(&what, dialect::NO_INLINE) + "; a plain def is called instead of expanded");
    }

    /// A conversion the program uses, found by a search at `span`: one from a library, since
    /// the program's own conversions are rejected where they are defined.
    pub fn restrict_conversion_use(&mut self, conversion: SymId, span: Span) {
        if !self.dialect.no_implicit_conversions || !self.restricted() {
            return;
        }
        let what = format!("the implicit conversion `{}` this expression needs", self.name_str(self.syms.sym(conversion).name));
        self.error(span, rejected(&what, dialect::NO_IMPLICIT_CONVERSIONS));
    }

    /// A `return` that leaves a function literal on its way out of the method.
    pub fn restrict_nonlocal_return(&mut self, span: Span) -> bool {
        if !self.dialect.no_nonlocal_returns || !self.restricted() {
            return false;
        }
        self.error(span, rejected("a return from inside a function literal", dialect::NO_NONLOCAL_RETURNS) + "; use `boundary` and `boundary.break` in `scala.util`");
        true
    }
}
