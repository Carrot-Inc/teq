//! Named tuples (Scala 3.7): `(name: T, ...)` is the std's opaque `NamedTuple[("name", ...),
//! (T, ...)]`, the tuple of the values at run time on every target. The typer supplies what the
//! language defines over that type: the literal, selection by name, the pattern, and the
//! conformance between a named tuple and the tuple it erases to.
use super::apply::{ArgList, Callee};
use super::Worker;
use crate::ast::{ExprId, ListRef, Pat, PatId};
use crate::intern::Name;
use crate::source::Span;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;

impl<'a> Worker<'a> {
    /// `scala.NamedTuple.NamedTuple`, the opaque type inside the object of that name.
    pub(super) fn find_named_tuple(&mut self) -> Option<ClassId> {
        let name = self.interner.intern("NamedTuple");
        let scala = self.b.scala_pkg;
        let term = match self.syms.pkg(scala).entries.get(&name).and_then(|e| e.term) {
            Some(t) => t,
            // scala-library's, which the class path holds.
            None if self.loaded.is_some() => match self.pkg_term(scala, name)? {
                super::resolve::TermRef::Global(s) => s,
                _ => return None,
            },
            None => return None,
        };
        let SymKind::Object(o) = self.syms.sym(term).kind else { return None };
        self.complete_class(o);
        self.syms.class(o).nested.get(&name).copied()
    }

    pub fn named_tuple_type(&mut self, names: &[Name], values: &[TypeId], span: Span) -> TypeId {
        let Some(nt) = self.named_tuple_class() else {
            self.error(span, "named tuples need the standard library's NamedTuple");
            return ERROR;
        };
        let lits: Vec<TypeId> = names.iter().map(|&n| self.types.lit(LitVal::Str(n))).collect();
        let n = self.tuple_type(&lits);
        let v = self.tuple_type(values);
        self.types.class(nt, &[n, v])
    }

    /// The names and value types of a named tuple type, when `t` is one with both settled.
    pub fn named_tuple_parts(&mut self, t: TypeId) -> Option<(Vec<Name>, Vec<TypeId>)> {
        let nt = self.named_tuple_class()?;
        let t = self.deref(t);
        let Type::Class(c, args) = self.types.get(t) else { return None };
        if c != nt {
            return None;
        }
        let [n, v] = *self.types.items(args) else { return None };
        let (n, v) = (self.deref(n), self.deref(v));
        let (Type::Class(nc, nargs), Type::Class(vc, vargs)) = (self.types.get(n), self.types.get(v)) else { return None };
        if !self.is_tuple_class(nc) || !self.is_tuple_class(vc) {
            return None;
        }
        let mut names = Vec::new();
        for &l in self.types.items(nargs).to_vec().iter() {
            let l = self.deref(l);
            match self.types.get(l) {
                Type::Lit(id) => match self.types.lit_val(id) {
                    LitVal::Str(s) => names.push(s),
                    _ => return None,
                },
                _ => return None,
            }
        }
        let values = self.types.items(vargs).to_vec();
        (names.len() == values.len()).then_some((names, values))
    }

    /// `(a = x, b = y)`: the tuple of the values, typed as the named tuple; against an expected
    /// named tuple of the same names the values take its element types.
    pub fn type_named_tuple(&mut self, names: ListRef, values: ListRef, expected: Option<TypeId>, span: Span) -> (TExprId, TypeId) {
        let ast = self.cur_ast();
        let names: Vec<Name> = ast.name_lists[names.range()].to_vec();
        let ids: Vec<ExprId> = ast.expr_list(values).to_vec();
        let expected_tuple = match expected.and_then(|e| self.concrete_expected(Some(e))) {
            Some(e) => match self.named_tuple_parts(e) {
                Some((n, v)) if n == names => Some(self.tuple_type(&v)),
                _ => None,
            },
            None => None,
        };
        let (te, ty) = self.type_tuple(&ids, expected_tuple);
        let Type::Class(_, args) = self.types.get(ty) else { return (te, ERROR) };
        let elems = self.types.items(args).to_vec();
        let ty = self.named_tuple_type(&names, &elems, span);
        (te, ty)
    }

    /// `x.name` on a named tuple: the field of the tuple at the name's position.
    pub fn named_tuple_member(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        targs: Option<ListRef>,
        lists: &[ArgList],
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        let (names, values) = self.named_tuple_parts(recv_ty)?;
        let i = names.iter().position(|&n| n == name)?;
        let tc = self.tuple_class(values.len());
        let fields = self.syms.class(tc).ctor_syms.concat();
        let te = self.tuple_element(recv, &fields, &values, i, span);
        if self.capturing() {
            self.capture_form(te, crate::tir::capture::Form::Op(name));
        }
        let ty = values[i];
        self.prog.set_type(te, ty);
        if lists.is_empty() && targs.is_none() {
            return Some((te, ty));
        }
        Some(self.apply_callee(Callee::Value(te, ty), targs, lists.to_vec(), span, expected))
    }

    /// `case (a = p, b = q)`: the fields named match their patterns and the others anything, in
    /// any order and as a subset, as scalac has it.
    pub fn named_tuple_pattern(&mut self, subs: &[PatId], sty: TypeId, span: Span) -> TPatId {
        let Some((names, values)) = self.named_tuple_parts(sty) else {
            let msg = format!("named patterns need a named tuple, found {}", self.show(sty));
            self.error(span, msg);
            return self.wildcard_after(subs);
        };
        let tc = self.tuple_class(values.len());
        let field_syms: Vec<SymId> = self.syms.class(tc).ctor_syms.concat();
        let vty = self.tuple_type(&values);
        let pats = self.patterns_by_name(&names, &values, subs, span, vty);
        if values.len() > 22 {
            if let Some(p) = self.xxl_elements_pattern(vty, &pats, sty, span) {
                return p;
            }
        }
        let fl = self.prog.syms(&field_syms);
        let pl = self.prog.pat_lists.push_slice(&pats);
        self.prog.add_pat(TPat::Class(tc, vty, fl, pl))
    }

    /// `case Person(name = n)`: the fields of a case class matched by name.
    pub(super) fn named_class_pattern(&mut self, c: ClassId, class_ty: TypeId, subst: &Subst, fields: &[ParamSig], subs: &[PatId], span: Span) -> TPatId {
        let names: Vec<Name> = fields.iter().map(|f| f.name).collect();
        let values: Vec<TypeId> = fields.iter().map(|f| self.types.subst(f.ty, subst)).collect();
        let field_syms: Vec<SymId> = fields.iter().map(|f| f.sym).collect();
        let pats = self.patterns_by_name(&names, &values, subs, span, class_ty);
        let fl = self.prog.syms(&field_syms);
        let pl = self.prog.pat_lists.push_slice(&pats);
        self.prog.add_pat(TPat::Class(c, class_ty, fl, pl))
    }

    fn patterns_by_name(&mut self, names: &[Name], values: &[TypeId], subs: &[PatId], span: Span, shown: TypeId) -> Vec<TPatId> {
        let ast = self.cur_ast();
        let mut pats: Vec<Option<TPatId>> = vec![None; names.len()];
        for &s in subs {
            let Pat::NamedField(n, p) = ast.pat(s) else {
                self.error(span, "Illegal combination of named and unnamed tuple elements");
                self.type_pattern(s, ERROR);
                continue;
            };
            match names.iter().position(|&m| m == n) {
                Some(i) if pats[i].is_none() => pats[i] = Some(self.type_pattern(p, values[i])),
                Some(_) => {
                    let msg = format!("{} is matched twice", self.name_str(n));
                    self.error(span, msg);
                    self.type_pattern(p, ERROR);
                }
                None => {
                    let msg = format!("{} is not a field of {}", self.name_str(n), self.show(shown));
                    self.error(span, msg);
                    self.type_pattern(p, ERROR);
                }
            }
        }
        let mut out = Vec::with_capacity(pats.len());
        for p in pats {
            out.push(match p {
                Some(p) => p,
                None => self.prog.add_pat(TPat::Wildcard),
            });
        }
        out
    }

    fn wildcard_after(&mut self, subs: &[PatId]) -> TPatId {
        let ast = self.cur_ast();
        for &s in subs {
            let inner = match ast.pat(s) {
                Pat::NamedField(_, p) => p,
                _ => s,
            };
            self.type_pattern(inner, ERROR);
        }
        self.prog.add_pat(TPat::Wildcard)
    }

    /// A plain tuple conforms to a named tuple of its element types under any names, as scalac
    /// has it (the join of a named tuple and a plain one is the named one); two named tuples
    /// are compared as the class's arguments are, so the names have to agree.
    pub(super) fn named_tuple_sub(&mut self, a: TypeId, b: TypeId) -> Option<bool> {
        let nt = self.named_tuple_class()?;
        let (ta, tb) = (self.types.get(a), self.types.get(b));
        match (ta, tb) {
            (Type::Class(ca, _), Type::Class(cb, bargs)) if cb == nt && ca != nt && self.is_tuple_class(ca) => {
                let v = self.types.items(bargs)[1];
                Some(self.is_sub(a, v))
            }
            // A named tuple is a `Tuple`, as the tuple it erases to is (`a *: (b: Any)`).
            (Type::Class(ca, aargs), Type::Class(cb, _))
                if ca == nt && (Some(cb) == self.b.tuple_trait || Some(cb) == self.b.non_empty_tuple || Some(cb) == self.b.cons_tuple) =>
            {
                let v = self.types.items(aargs)[1];
                Some(self.is_sub(v, b))
            }
            _ => None,
        }
    }

    /// A named tuple where a plain tuple is expected is the tuple it erases to, as scalac
    /// adapts it (`val back: (Int, String) = p`).
    pub(super) fn named_to_tuple(&mut self, te: TExprId, actual: TypeId, expected: TypeId) -> Option<TExprId> {
        let (_, values) = self.named_tuple_parts(actual)?;
        let exp = self.deref(expected);
        let Type::Class(c, _) = self.types.get(exp) else { return None };
        if !self.is_tuple_class(c) {
            return None;
        }
        let tuple = self.tuple_type(&values);
        let mark = self.snapshot();
        if self.is_sub(tuple, expected) {
            return Some(te);
        }
        self.rollback(mark);
        None
    }

    /// The scrutinee type the patterns of a match are checked against: a named tuple is
    /// matched as the tuple it erases to.
    pub fn erased_named_tuple(&mut self, t: TypeId) -> TypeId {
        match self.named_tuple_parts(t) {
            Some((_, values)) => self.tuple_type(&values),
            None => t,
        }
    }

    /// Writes `(a: Int, b: String)` for the arguments of a `NamedTuple`; false when they are
    /// not settled tuples of literal names and types.
    pub(super) fn show_named_tuple(&self, items: &[TypeId], out: &mut String) -> bool {
        let [n, v] = *items else { return false };
        let (Type::Class(nc, nargs), Type::Class(vc, vargs)) = (self.types.get(n), self.types.get(v)) else { return false };
        if !self.is_tuple_class(nc) || !self.is_tuple_class(vc) {
            return false;
        }
        let (names, values) = (self.types.items(nargs).to_vec(), self.types.items(vargs).to_vec());
        if names.len() != values.len() {
            return false;
        }
        let mut text = String::from("(");
        for (i, (&l, &t)) in names.iter().zip(&values).enumerate() {
            let Type::Lit(id) = self.types.get(l) else { return false };
            let LitVal::Str(name) = self.types.lit_val(id) else { return false };
            if i > 0 {
                text.push_str(", ");
            }
            text.push_str(self.interner.get(name));
            text.push_str(": ");
            self.show_into(t, &mut text, false);
        }
        text.push(')');
        out.push_str(&text);
        true
    }
}
