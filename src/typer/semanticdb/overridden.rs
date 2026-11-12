//! The members a member overrides, for `teq fix`'s ForbiddenSymbol (docs/FIX.md, "The rules teq ships"): the port of
//! dotty 3.8.4's `SymDenotations.allOverriddenSymbols` (SymDenotations.scala 1451-1453), which maps `overriddenSymbol`
//! (1446-1448) over the base classes of the member's owner, each giving `matchingDecl` (1417-1422): the non-private
//! declaration of the member's name in that class whose type matches the member's. The match is the typer's own of
//! two inherited members (`inherited_members_match`, dotty's `Denotation.matches`), which the override checks use.

use super::super::Worker;
use crate::symbols::*;
use crate::types::*;

impl<'a> Worker<'a> {
    /// `allOverriddenSymbols` of `s`, in the order of its owner's base classes.
    pub(super) fn sem_overridden(&mut self, s: SymId) -> Vec<SymId> {
        let (owner, name, kind, mods, is_ext) = {
            let info = self.syms.sym(s);
            (info.owner, info.name, info.kind, info.mods, info.is_extension)
        };
        // `canMatchInheritedSymbols`: a member of a class that is no constructor and not private.
        let Owner::Class(c) = owner else { return Vec::new() };
        if mods & crate::ast::mods::PRIVATE != 0 || !matches!(kind, SymKind::Def | SymKind::Val | SymKind::Var | SymKind::Given) {
            return Vec::new();
        }
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
        self.sig_of(s);
        let mut out = Vec::new();
        for &(b, b_ty) in bases.iter().skip(1) {
            let mut k = 0;
            while let Some(p) = self.inherited_member(b, name, is_ext, k) {
                k += 1;
                let declared = {
                    let info = self.syms.sym(p);
                    info.owner == Owner::Class(b) && info.mods & crate::ast::mods::PRIVATE == 0 && matches!(info.kind, SymKind::Def | SymKind::Val | SymKind::Var | SymKind::Given)
                };
                if !declared || p == s {
                    continue;
                }
                self.sig_of(p);
                if self.inherited_members_match(c, &bases, s, p, b_ty) {
                    out.push(p);
                    break;
                }
            }
        }
        out
    }
}
