//! The feature warning of a definition of an old-style implicit conversion, dotty's
//! `Checking.checkImplicitConversionDefOK` and `checkFeature`: an `implicit def` whose first
//! parameter list holds one parameter and is no implicit or using clause, written as such (the
//! conversion an implicit class makes is not), warns `Definition of implicit conversion method f
//! should be enabled`, at its name, unless the feature `implicitConversions` is enabled: by
//! `--language implicitConversions` (scalac's `-language:implicitConversions`) or by an
//! `import scala.language.implicitConversions` in scope (`Feature.enabledByImport`, the imports
//! the parser keeps with their scopes in `Ast::language_scopes`). A `FeatureWarning`, which the
//! reporting policy summarizes unless `--feature`; the first of each top-level definition, the
//! use site scalac's reporter remembers (`isReportedFeatureUseSite`), explains the feature
//! (`Diagnostics::report`).

use super::Worker;
use crate::ast::{self, mods, DefKind};
use crate::intern::Name;
use crate::source::{FileId, Span};

/// The explanation the first feature warning of a use site adds (`report.featureWarning`).
pub fn explanation(feature: &str) -> String {
    format!("\nSee the Scala docs for value scala.language.{} for a discussion\nwhy the feature should be explicitly enabled.", feature)
}

impl<'a> Worker<'a> {
    /// The definition `d` of `file`, checked as an implicit conversion's.
    #[inline]
    pub(super) fn check_conversion_def(&mut self, file: FileId, d: ast::DefId) {
        if self.ast(file).def(d).mods & mods::IMPLICIT != 0 {
            self.check_implicit_def(file, d);
        }
    }

    /// An implicit definition's check, apart from the common path.
    #[cold]
    #[inline(never)]
    fn check_implicit_def(&mut self, file: FileId, d: ast::DefId) {
        let ast = self.ast(file);
        let def = ast.def(d);
        if !self.program_source(file) {
            return;
        }
        let DefKind::Fun(f) = &def.kind else { return };
        if f.is_extension || !matches!(f.clauses.first(), Some(c) if c.params.len() == 1 && !c.is_using && !c.is_implicit) {
            return;
        }
        // The conversion of an implicit class, which the parser makes beside it at its place.
        let made = ast.defs.iter().any(|k| matches!(k.kind, DefKind::Class(_)) && k.name == def.name && k.span == def.span && k.mods & mods::IMPLICIT != 0);
        if made || self.feature_enabled(file, def.span.start, crate::names::IMPLICIT_CONVERSIONS) {
            return;
        }
        let use_site = self.top_level_def_of(file, d);
        let msg = format!(
            "Definition of implicit conversion method {} should be enabled\nby adding the import clause 'import scala.language.implicitConversions'\nor by setting the compiler option --language implicitConversions.",
            self.name_str(def.name)
        );
        let w = crate::source::Warning { id: crate::warnings::NO_ID, category: crate::warnings::Category::Feature, origin: Some(format!("{}:{}", file.0, use_site).into_boxed_str()), phase: 0, unplaced: false };
        let file_was = std::mem::replace(&mut self.env.file, file);
        self.warn_as(Span::new(def.span.start, def.span.start + 1), msg, w);
        self.env.file = file_was;
    }

    /// Whether `feature` is enabled at `pos` of `file`: by the command line, or by the innermost
    /// `scala.language` import in scope there that names it or all features.
    pub(super) fn feature_enabled(&self, file: FileId, pos: u32, feature: Name) -> bool {
        if self.diags.policy.language.contains(&self.name_str(feature)) {
            return true;
        }
        // The innermost clause that names it decides (`ImportInfo.featureImported`): the latest
        // to start among those in scope, of one statement its last clause.
        let ast = self.ast(file);
        ast.language_scopes
            .iter()
            .filter(|(n, _, scope)| *n == feature && scope.start <= pos && pos < scope.end)
            .max_by_key(|(_, _, scope)| scope.start)
            .is_some_and(|&(_, enables, _)| enables)
    }

    /// The top-level class of `file` that holds `d`, scalac's `ctx.owner.topLevelClass`: its
    /// index among the definitions, or `u32::MAX` for the file's package object, which holds its
    /// top-level methods and values.
    fn top_level_def_of(&self, file: FileId, d: ast::DefId) -> u32 {
        let ast = self.ast(file);
        let at = ast.def_range(d);
        ast.top_level
            .iter()
            .find(|&&t| {
                let r = ast.def_range(t);
                matches!(ast.def(t).kind, DefKind::Class(_)) && r.start <= at.start && at.end <= r.end
            })
            .map_or(u32::MAX, |t| t.0)
    }
}
