//! Tuple and function types of any arity. `TupleN` is a case class with the fields `_1` to
//! `_N` and `FunctionN` a trait with an abstract `apply`; both are made on first use, so the
//! standard library holds no per-arity source and the program only pays for the arities it uses.
//! Past 22 elements a tuple is scalac's `scala.runtime.TupleXXL` at run time: `TupleN` stays the
//! type, its fields no members, and the values are made, read and matched through TupleXXL
//! (`tuple_xxl_value`, `tuple_element`, `pat.rs`'s `xxl_tuple_pattern`).

use super::Worker;
use crate::ast::mods;
use crate::intern::Name;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::sync::Arc;

/// The members of `Tuple` the compiler types on a receiver whose static type is a tuple type
/// (`tuple_member`), in `Builtins::tuple_members`' order.
pub(crate) const TUPLE_MEMBERS: [&str; 14] = ["head", "tail", "size", "apply", "++", "zip", "toList", "last", "init", "reverse", ":*", "take", "drop", "splitAt"];

#[derive(Clone, Copy)]
enum ArityTable {
    Tuples,
    Functions,
    ContextFunctions,
}

impl<'a> Worker<'a> {
    pub fn tuple_class(&mut self, n: usize) -> ClassId {
        match self.b.tuples.get(n).copied().flatten() {
            Some(c) => c,
            None => self.synthesize_tuple(n),
        }
    }

    pub fn function_class(&mut self, n: usize) -> ClassId {
        match self.b.functions.get(n).copied().flatten() {
            Some(c) => c,
            None => self.synthesize_function(n, false),
        }
    }

    /// The type of a by-name parameter of a function type, `=> A` in `(=> A) => B`: an opaque
    /// type over `() => A`, the thunk that is passed, which no other type conforms to.
    pub fn by_name_class(&mut self) -> ClassId {
        self.with_loader(|w| w.by_name_class_unlocked())
    }

    pub fn by_name_class_unlocked(&mut self) -> ClassId {
        if let Some(c) = self.b.by_name {
            return c;
        }
        let scala = self.b.scala_pkg;
        let name = self.interner.intern("<byname>");
        if let Some(&c) = self.arity_classes.get(&name) {
            self.b.by_name = Some(c);
            return c;
        }
        let c = self.syms.new_class(name, ClassKind::Opaque, mods::FINAL, Owner::Package(scala), FileId(0), None, Span::default());
        let a = self.interner.intern("A");
        let a = self.syms.new_tparam(a, 1);
        let at = self.types.param(a);
        let thunk = self.function_class(0);
        let underlying = self.types.class(thunk, &[at]);
        let self_ty = self.types.class(c, &[at]);
        let mut info = self.syms.class_mut(c);
        info.tparams = vec![a];
        info.underlying = Some(underlying);
        info.base_types = vec![(c, self_ty)];
        info.state().set(Completion::Done);
        self.b.by_name = Some(c);
        self.arity_classes.insert(name, c);
        c
    }

    /// scalac's `<repeated>[A]`, the type reflection gives a repeated parameter `xs: A*`: an
    /// opaque type over `Seq[A]`, made once and shared between the workers as the arity
    /// classes are.
    pub fn repeated_type(&mut self, t: TypeId) -> TypeId {
        let c = match self.b.repeated {
            Some(c) => c,
            None => self.with_loader(|w| w.repeated_class_unlocked()),
        };
        self.types.class(c, &[t])
    }

    fn repeated_class_unlocked(&mut self) -> ClassId {
        if let Some(c) = self.b.repeated {
            return c;
        }
        let scala = self.b.scala_pkg;
        let name = self.interner.intern("<repeated>");
        if let Some(&c) = self.arity_classes.get(&name) {
            self.b.repeated = Some(c);
            return c;
        }
        let c = self.syms.new_class(name, ClassKind::Opaque, mods::FINAL, Owner::Package(scala), FileId(0), None, Span::default());
        let a = self.interner.intern("A");
        let a = self.syms.new_tparam(a, 1);
        let at = self.types.param(a);
        let underlying = match self.seq_class() {
            Some(seq) => self.types.class(seq, &[at]),
            None => ANY,
        };
        let self_ty = self.types.class(c, &[at]);
        let mut info = self.syms.class_mut(c);
        info.tparams = vec![a];
        info.underlying = Some(underlying);
        info.base_types = vec![(c, self_ty)];
        info.state().set(Completion::Done);
        self.b.repeated = Some(c);
        self.arity_classes.insert(name, c);
        c
    }

    /// `A` of a repeated parameter's reflected type `<repeated>[A]`.
    pub fn repeated_arg(&mut self, t: TypeId) -> Option<TypeId> {
        self.sync_arity_if_stale();
        let c = self.b.repeated?;
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(k, args) if k == c => Some(self.types.items(args)[0]),
            _ => None,
        }
    }

    pub fn by_name_type(&mut self, t: TypeId) -> TypeId {
        let c = self.by_name_class();
        self.types.class(c, &[t])
    }

    /// `A` of a by-name function parameter's type `=> A`.
    #[inline]
    pub fn by_name_arg(&mut self, t: TypeId) -> Option<TypeId> {
        self.sync_arity_if_stale();
        let c = self.b.by_name?;
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(k, args) if k == c => Some(self.types.items(args)[0]),
            _ => None,
        }
    }

    pub fn context_function_class(&mut self, n: usize) -> ClassId {
        match self.b.context_functions.get(n).copied().flatten() {
            Some(c) => c,
            None => self.synthesize_function(n, true),
        }
    }

    pub fn is_context_function_class(&self, c: ClassId) -> bool {
        let n = self.syms.class(c).tparams.len();
        n > 0 && (self.b.context_functions.get(n - 1) == Some(&Some(c)) || self.registered_arity_class(c, "ContextFunction"))
    }

    pub fn is_tuple_class(&self, c: ClassId) -> bool {
        let n = self.syms.class(c).tparams.len();
        self.b.tuples.get(n) == Some(&Some(c)) || self.registered_arity_class(c, "Tuple")
    }

    pub fn is_function_class(&self, c: ClassId) -> bool {
        let n = self.syms.class(c).tparams.len();
        n > 0 && (self.b.functions.get(n - 1) == Some(&Some(c)) || self.registered_arity_class(c, "Function"))
    }

    /// Whether `c` is an arity class another worker made (`arity_classes`, the cell) of the
    /// kind `prefix` names, which this worker's own tables lack until `sync_arity_tables`.
    fn registered_arity_class(&self, c: ClassId, prefix: &str) -> bool {
        if !self.forked {
            return false;
        }
        let name = self.syms.class(c).name;
        let s = self.name_ref(name);
        s.starts_with(prefix) && s[prefix.len()..].bytes().all(|b| b.is_ascii_digit()) && self.arity_classes.get(&name) == Some(&c)
    }

    /// `sync_arity_tables` where a reader of the tables is about to miss: two counters
    /// compared when nothing is new. A signature another worker completed names the arity
    /// classes that worker made, whether this worker waited for it or found it done.
    #[inline]
    pub fn sync_arity_if_stale(&mut self) {
        if self.forked && self.arity_seen < self.arity_classes.shared_len() {
            self.sync_arity_tables();
        }
    }

    /// Puts the arity classes other workers made under this worker's tables, so that the
    /// reads of `b.functions`, `b.tuples`, `b.context_functions` and `b.by_name` see them.
    pub fn sync_arity_tables(&mut self) {
        let n = self.arity_classes.shared_len();
        while self.arity_seen < n {
            let (&name, &c) = self.arity_classes.shared_entry_at(self.arity_seen);
            self.arity_seen += 1;
            let s = self.name_ref(name).to_string();
            let numbered = |s: &str, prefix: &str| s.strip_prefix(prefix).and_then(|d| d.parse::<usize>().ok());
            if s == "<byname>" {
                self.b.by_name.get_or_insert(c);
            } else if s == "<repeated>" {
                self.b.repeated.get_or_insert(c);
            } else if let Some(k) = numbered(&s, "ContextFunction") {
                self.note_arity_class(k, c, ArityTable::ContextFunctions);
            } else if let Some(k) = numbered(&s, "Function") {
                self.note_arity_class(k, c, ArityTable::Functions);
            } else if let Some(k) = numbered(&s, "Tuple") {
                self.note_arity_class(k, c, ArityTable::Tuples);
            }
        }
    }

    /// `TupleN` and `FunctionN` looked up by name in package `scala`.
    #[cold]
    pub fn arity_class_named(&mut self, name: Name) -> Option<ClassId> {
        let text = self.interner.get(name);
        let (prefix, table) = match text.strip_prefix("Tuple") {
            Some(rest) => (rest, ArityTable::Tuples),
            None => match text.strip_prefix("ContextFunction") {
                Some(rest) => (rest, ArityTable::ContextFunctions),
                None => (text.strip_prefix("Function")?, ArityTable::Functions),
            },
        };
        if prefix.is_empty() || prefix.len() > 4 || !prefix.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        if prefix.len() > 1 && prefix.starts_with('0') {
            return None;
        }
        let n: usize = prefix.parse().ok()?;
        Some(match table {
            ArityTable::Tuples => {
                if n == 0 {
                    return None;
                }
                self.tuple_class(n)
            }
            ArityTable::Functions => self.function_class(n),
            ArityTable::ContextFunctions => self.context_function_class(n),
        })
    }

    #[cold]
    fn synthesize_tuple(&mut self, n: usize) -> ClassId {
        self.with_loader(|w| w.synthesize_tuple_unlocked(n))
    }

    fn synthesize_tuple_unlocked(&mut self, n: usize) -> ClassId {
        let scala = self.b.scala_pkg;
        let name = self.interner.intern(&format!("Tuple{}", n));
        if let Some(&c) = self.arity_classes.get(&name) {
            self.note_arity_class(n, c, ArityTable::Tuples);
            return c;
        }
        let c = self.syms.new_class(
            name,
            ClassKind::Class,
            mods::FINAL | mods::CASE,
            Owner::Package(scala),
            FileId(0),
            None,
            Span::default(),
        );
        let mut tparams = Vec::with_capacity(n);
        let mut params = Vec::with_capacity(n);
        let mut syms = Vec::with_capacity(n);
        for i in 1..=n {
            let tp = self.interner.intern(&format!("T{}", i));
            let tp = self.syms.new_tparam(tp, 1);
            tparams.push(tp);
            let ty = self.types.param(tp);
            let field = self.interner.intern(&format!("_{}", i));
            let sym = self.syms.new_sym(field, SymKind::Val, mods::FIELD, Owner::Class(c), FileId(0), None, Span::default());
            let sig = self.value_sig(ty);
            let mut info = self.syms.sym_mut(sym);
            info.sig = Some(sig);
            info.state().set(Completion::Done);
            // Past 22 elements a tuple is scalac's TupleXXL, which has no `_N`: scalac rejects
            // the selection, and the compiler reads the elements by index (`tuple_element`).
            if n <= 22 {
                self.syms.add_member(c, sym);
            }
            params.push(ParamSig { name: field, ty, by_name: false, repeated: false, has_default: false, sym });
            syms.push(sym);
        }
        let targs: Vec<TypeId> = tparams.iter().map(|&p| self.types.param(p)).collect();
        let self_ty = self.types.class(c, &targs);
        // The supertypes teq's own std declares; under `--std=scala-library` the library's come through
        // `link_library_class`, and nothing is read from the classpath here.
        let mut base_types = vec![(c, self_ty)];
        for parent in ["NonEmptyTuple", "Tuple"] {
            let name = self.interner.intern(parent);
            if let Some(p) = self.syms.pkg(scala).entries.get(&name).and_then(|e| e.class) {
                let pt = self.types.class(p, &[]);
                base_types.push((p, pt));
            }
        }
        let mut info = self.syms.class_mut(c);
        info.tparams = tparams;
        info.ctor = vec![ClauseSig { params, is_using: false, is_implicit: false }];
        info.ctor_syms = vec![syms];
        info.base_types = base_types;
        info.state().set(Completion::Done);
        self.register_arity_class(n, c, ArityTable::Tuples);
        c
    }

    /// `FunctionN`, or `ContextFunctionN` whose `apply` takes its parameters as a using clause.
    #[cold]
    fn synthesize_function(&mut self, n: usize, context: bool) -> ClassId {
        self.with_loader(|w| w.synthesize_function_unlocked(n, context))
    }

    fn synthesize_function_unlocked(&mut self, n: usize, context: bool) -> ClassId {
        let scala = self.b.scala_pkg;
        let name = self.interner.intern(&format!("{}Function{}", if context { "Context" } else { "" }, n));
        if let Some(&c) = self.arity_classes.get(&name) {
            self.note_arity_class(n, c, if context { ArityTable::ContextFunctions } else { ArityTable::Functions });
            return c;
        }
        let c = self.syms.new_class(name, ClassKind::Trait, 0, Owner::Package(scala), FileId(0), None, Span::default());
        let mut tparams = Vec::with_capacity(n + 1);
        let mut params = Vec::with_capacity(n);
        for i in 1..=n {
            let tp = self.interner.intern(&format!("T{}", i));
            let tp = self.syms.new_tparam(tp, -1);
            tparams.push(tp);
            let ty = self.types.param(tp);
            let pname = self.interner.intern(&format!("v{}", i));
            let sym = self.syms.new_sym(pname, SymKind::Param, 0, Owner::Local, FileId(0), None, Span::default());
            let sig = self.value_sig(ty);
            let mut info = self.syms.sym_mut(sym);
            info.sig = Some(sig);
            info.state().set(Completion::Done);
            params.push(ParamSig { name: pname, ty, by_name: false, repeated: false, has_default: false, sym });
        }
        let result = self.interner.intern("R");
        let result = self.syms.new_tparam(result, 1);
        tparams.push(result);
        let ret = self.types.param(result);
        let apply = self.syms.new_sym(
            crate::names::APPLY,
            SymKind::Def,
            crate::ast::mods::ABSTRACT,
            Owner::Class(c),
            FileId(0),
            None,
            Span::default(),
        );
        {
            let sig = MethodSig { tparams: Vec::new(), clauses: vec![ClauseSig { params, is_using: context, is_implicit: false }], ret };
            let mut info = self.syms.sym_mut(apply);
            info.sig = Some(Arc::new(sig));
            info.state().set(Completion::Done);
        }
        self.syms.add_member(c, apply);
        let targs: Vec<TypeId> = tparams.iter().map(|&p| self.types.param(p)).collect();
        let self_ty = self.types.class(c, &targs);
        let mut info = self.syms.class_mut(c);
        info.tparams = tparams;
        info.base_types = vec![(c, self_ty)];
        info.state().set(Completion::Done);
        self.register_arity_class(n, c, if context { ArityTable::ContextFunctions } else { ArityTable::Functions });
        c
    }

    /// `f.tupled` and `f.curried` of a function of two or more parameters, which Scala defines on
    /// every `FunctionN`: `p => f(p._1, .., p._n)` and `a1 => .. => an => f(a1, .., an)`, with `f`
    /// evaluated once.
    pub fn function_conversion(
        &mut self,
        recv: TExprId,
        params: &[TypeId],
        ret: TypeId,
        name: Name,
        span: Span,
    ) -> (TExprId, TypeId) {
        let fun_ty = self.fun_type(params, ret);
        let f = self.fresh_local("f", fun_ty, span);
        let f_ref = self.prog.add(TExpr::Local(f));
        let (lambda, ty) = if name == crate::names::TUPLED {
            let tuple_ty = self.tuple_type(params);
            let tuple_class = self.tuple_class(params.len());
            let p = self.fresh_local("p", tuple_ty, span);
            let fields = self.syms.class(tuple_class).ctor_syms[0].clone();
            let args: Vec<TExprId> = fields
                .iter()
                .map(|&field| {
                    let p_ref = self.prog.add(TExpr::Local(p));
                    self.prog.add(TExpr::Field(p_ref, field))
                })
                .collect();
            let args = self.prog.list(&args);
            let call = self.prog.add(TExpr::CallClosure(f_ref, args));
            let ps = self.prog.syms(&[p]);
            (self.prog.add(TExpr::Lambda(ps, call)), self.fun_type(&[tuple_ty], ret))
        } else {
            let locals: Vec<SymId> =
                params.iter().enumerate().map(|(i, &t)| self.indexed_local("a", i as u32, t, span)).collect();
            let args: Vec<TExprId> = locals.iter().map(|&s| self.prog.add(TExpr::Local(s))).collect();
            let args = self.prog.list(&args);
            let mut body = self.prog.add(TExpr::CallClosure(f_ref, args));
            let mut ty = ret;
            for (&local, &t) in locals.iter().zip(params).rev() {
                let ps = self.prog.syms(&[local]);
                body = self.prog.add(TExpr::Lambda(ps, body));
                ty = self.fun_type(&[t], ty);
            }
            (body, ty)
        };
        let stmts = self.prog.stmts.push_slice(&[TStmt::Val(f, recv)]);
        (self.prog.add(TExpr::Block(stmts, lambda)), ty)
    }

    /// The members Scala gives every tuple through the match types of `Tuple`, on a receiver
    /// whose static type is a tuple: `head`, `tail`, `size`, `apply(n)` with a literal `n`,
    /// `++` and `zip` with a tuple argument. Each is the field access or the new tuple it
    /// stands for, typed as the match type reduces.
    pub(super) fn tuple_member(
        &mut self,
        recv: TExprId,
        recv_ty: TypeId,
        name: Name,
        lists: &[super::apply::ArgList],
        span: Span,
    ) -> Option<(TExprId, TypeId)> {
        if !self.b.may_be_tuple_member(name) {
            return None;
        }
        let which = self.b.tuple_members.iter().position(|&n| n == name)?;
        let text = TUPLE_MEMBERS[which];
        // The argument lists past the member's own are its result's (`t.init(21)`, `t.take(2)(1)`).
        let own = match text {
            "head" | "tail" | "size" | "toList" | "last" | "init" | "reverse" => 0,
            _ => lists.len().min(1),
        };
        let (lists, rest) = lists.split_at(own);
        // The receiver's head says whether it can be a tuple at all, before it is dealiased.
        let head = self.deref(recv_ty);
        match self.types.get(head) {
            Type::Class(c, _) => {
                if !self.is_tuple_class(c) && Some(c) != self.b.cons_tuple && Some(c) != self.b.empty_tuple && self.syms.class(c).kind != ClassKind::Opaque {
                    return None;
                }
            }
            Type::Match(..) | Type::Alias(..) | Type::Member(..) | Type::AppMember(..) | Type::Decl(_) | Type::Term(_) | Type::Select(..) | Type::Param(_) | Type::Inter(..) => {}
            _ => return None,
        }
        let elems = self.tuple_elements(recv_ty)?;
        let fields = self.tuple_fields(recv_ty);
        // Several arguments to `++` or `zip` are the tuple they make, as scalac adapts them. An
        // argument named as scala-library's parameter (`t.take(n = 1)`) is that argument; any other
        // named one leaves the call to the member's own application, which reports it.
        let param = match text {
            "apply" | "take" | "drop" | "splitAt" => "n",
            ":*" => "x",
            "++" => "that",
            "zip" => "t2",
            _ => "",
        };
        let one_arg = |t: &mut Self| -> Option<(TExprId, TypeId)> {
            let [list] = lists else { return None };
            if list.using || list.args.is_empty() {
                return None;
            }
            let ast = t.cur_ast();
            let named = |a: &super::apply::ArgSrc| match *a {
                super::apply::ArgSrc::Ast(e) | super::apply::ArgSrc::Hoisted(e) => match ast.expr(e) {
                    crate::ast::Expr::NamedArg(name, value) => Some((name, value)),
                    _ => None,
                },
                _ => None,
            };
            // An argument that does not type alone (a lambda without its parameter's type) leaves
            // the call to the member's application, where an extension of the name may take it.
            let (mark, diags) = (t.snapshot(), t.diags.items.len());
            let failed = |t: &mut Self| {
                if t.diags.items[diags..].iter().any(|d| !d.is_warning) {
                    t.discard_diagnostics(diags);
                    t.rollback(mark);
                    return true;
                }
                false
            };
            if list.args.iter().any(|a| named(a).is_some()) {
                let [arg] = list.args.as_slice() else { return None };
                let (name, value) = named(arg)?;
                if t.interner.get(name) != param {
                    return None;
                }
                let typed = t.type_expr(value, None);
                return (!failed(t)).then_some(typed);
            }
            let mut typed = Vec::with_capacity(list.args.len());
            for arg in &list.args {
                typed.push(match *arg {
                    super::apply::ArgSrc::Ast(e) | super::apply::ArgSrc::Hoisted(e) => t.type_expr(e, None),
                    super::apply::ArgSrc::Typed(te, ty) | super::apply::ArgSrc::Named(_, te, ty) => (te, ty),
                    super::apply::ArgSrc::ForLambda(..) => return None,
                });
            }
            if failed(t) {
                return None;
            }
            if let [(te, ty)] = typed[..] {
                return Some((te, ty));
            }
            let items: Vec<TExprId> = typed.iter().map(|&(te, _)| te).collect();
            let tys: Vec<TypeId> = typed.iter().map(|&(_, ty)| t.solve_inferred(ty)).collect();
            let ty = t.tuple_type(&tys);
            let tuple = t.tuple_value(&items);
            if t.capturing() && tys.len() <= 22 {
                t.capture_targs(tuple, &tys);
            }
            Some((tuple, ty))
        };
        let mark = self.hoisted.len();
        let mut arg_node = None;
        let result = match text {
            "head" if lists.is_empty() && !elems.is_empty() => (self.tuple_element(recv, &fields, &elems, 0, span), elems[0]),
            "tail" if lists.is_empty() && !elems.is_empty() => {
                let t = self.hoist(recv, recv_ty, span);
                let rest: Vec<TExprId> = (1..elems.len())
                    .map(|i| {
                        let t = self.copy_expr(t);
                        self.tuple_element(t, &fields, &elems, i, span)
                    })
                    .collect();
                let ty = self.tuple_of(&elems[1..]);
                (self.tuple_value(&rest), ty)
            }
            "last" if lists.is_empty() && !elems.is_empty() => {
                let i = elems.len() - 1;
                (self.tuple_element(recv, &fields, &elems, i, span), elems[i])
            }
            // A tuple of one element is its own reverse, as scalac's runtime returns it.
            "reverse" if lists.is_empty() && elems.len() == 1 => (recv, recv_ty),
            "init" | "reverse" if lists.is_empty() && (text == "reverse" || !elems.is_empty()) => {
                let order: Vec<usize> = if text == "init" { (0..elems.len() - 1).collect() } else { (0..elems.len()).rev().collect() };
                self.tuple_elements_of(recv, recv_ty, &fields, &elems, &order, span)
            }
            "take" | "drop" | "splitAt" => {
                let (arg, _) = one_arg(self)?;
                arg_node = Some(arg);
                let Some(LitVal::Int(k)) = self.fold_constant(arg) else { return None };
                let k = usize::try_from(k).ok()?.min(elems.len());
                let (front, back): (Vec<usize>, Vec<usize>) = ((0..k).collect(), (k..elems.len()).collect());
                match text {
                    "take" => self.tuple_elements_of(recv, recv_ty, &fields, &elems, &front, span),
                    "drop" => self.tuple_elements_of(recv, recv_ty, &fields, &elems, &back, span),
                    _ => {
                        let recv = self.hoist(recv, recv_ty, span);
                        let first = self.copy_expr(recv);
                        let (a, aty) = self.tuple_elements_of(first, recv_ty, &fields, &elems, &front, span);
                        let second = self.copy_expr(recv);
                        let (b, bty) = self.tuple_elements_of(second, recv_ty, &fields, &elems, &back, span);
                        (self.tuple_value(&[a, b]), self.tuple_type(&[aty, bty]))
                    }
                }
            }
            ":*" => {
                let (arg, arg_ty) = one_arg(self)?;
                arg_node = Some(arg);
                let arg_ty = self.solve_inferred(arg_ty);
                let arg_ty = self.widen_lit(arg_ty);
                let recv = self.hoist(recv, recv_ty, span);
                let arg = self.hoist(arg, arg_ty, span);
                let mut items: Vec<TExprId> = (0..elems.len())
                    .map(|i| {
                        let t = self.copy_expr(recv);
                        self.tuple_element(t, &fields, &elems, i, span)
                    })
                    .collect();
                items.push(self.copy_expr(arg));
                let mut tys = elems.clone();
                tys.push(arg_ty);
                let ty = self.tuple_of(&tys);
                (self.tuple_value(&items), ty)
            }
            "size" if lists.is_empty() => {
                let n = elems.len() as i32;
                (self.prog.add(TExpr::Int(n)), self.types.lit(LitVal::Int(n)))
            }
            "toList" if lists.is_empty() => {
                let (cons, nil, list) = (self.std_class("::")?, self.std_object("Nil")?, self.std_class("List")?);
                let t = self.hoist(recv, recv_ty, span);
                let mut value = self.prog.add(TExpr::Module(nil));
                for i in (0..elems.len()).rev() {
                    let t = self.copy_expr(t);
                    let head = self.tuple_element(t, &fields, &elems, i, span);
                    let l = self.prog.list(&[head, value]);
                    value = self.prog.add(TExpr::New(cons, l));
                }
                let elem = elems.iter().fold(NOTHING, |acc, &e| self.types.union(acc, e));
                (value, self.types.class(list, &[elem]))
            }
            "apply" => {
                let (arg, _) = one_arg(self)?;
                arg_node = Some(arg);
                // The index is static where scalac types it as a constant, which an inline
                // expansion's substitution keeps (`t(2)`, `t(1 + 1)`, `t(K.k)` of a `final val
                // k = 2`, `t(k)` of a `val k: 2`, an inline `def k: 2`, an inline parameter given
                // `2`, a `constValue`); a widened one (`t((2: Int))`, an inline `def k: Int`, a
                // `val`, a `def`, a block of effects) is read at run time, as scalac's
                // `Elem[T, Int]` does not reduce.
                if self.widened_constant(arg) {
                    return None;
                }
                // A plain call still pending of a constant type, the index or an operand of it
                // (`M.one + 0`), folds by its type, as scalac's typer reads it, and stays a call,
                // expanded in its order in the later phase; one of a
                // widened type folds to none, read at run time.
                let pending = self.holds_pending_call(arg);
                let folded = if pending { self.fold_by_pending_types(arg) } else { self.fold_constant(arg) };
                let Some(LitVal::Int(i)) = folded else { return None };
                // A literal past the elements is scalac's error, where `Elem` does not reduce. In an
                // inline method's body checked at its definition it is the expansion's to report,
                // as an `inline if` may never reach it (the deferred calls).
                let Some(k) = usize::try_from(i).ok().filter(|&k| k < elems.len()) else {
                    if self.checks_inline_definition() {
                        return self.deferred_index_error(i, span).or(None);
                    }
                    let at = match lists.first().and_then(|l| l.args.first()) {
                        Some(&super::apply::ArgSrc::Ast(e) | &super::apply::ArgSrc::Hoisted(e)) => self.cur_ast().expr_span(e),
                        _ => span,
                    };
                    self.error(at, format!("index out of bounds: {}", i));
                    return Some((recv, ERROR));
                };
                if !pending {
                    (self.tuple_element(recv, &fields, &elems, k, span), elems[k])
                } else {
                    // The index is evaluated after the receiver and before the element is read, for
                    // what its expansions do, and dropped once they leave it a constant
                    // (`fold_indexes_later`).
                    // A module's reference may initialize it, a val of the program read through it.
                    let inert = |t: &Self, e: TExprId| t.is_stable(e) && !matches!(t.prog.expr(e), TExpr::Module(_) | TExpr::Static(_));
                    let evaluates_nothing = inert(self, recv)
                        || match self.prog.expr(recv) {
                            TExpr::New(c, args) if self.is_tuple_class(c) => self.prog.expr_list(args).iter().all(|&a| inert(self, a)),
                            _ => false,
                        };
                    let recv = if evaluates_nothing { recv } else { self.hoist(recv, recv_ty, span) };
                    let elem = self.tuple_element(recv, &fields, &elems, k, span);
                    let stmts = self.prog.stmts.push_slice(&[TStmt::Expr(arg)]);
                    let te = self.prog.add(TExpr::Block(stmts, elem));
                    self.attempts.index_later.push((arg, te, elem));
                    (te, elems[k])
                }
            }
            "++" | "zip" => {
                let (arg, arg_ty) = one_arg(self)?;
                arg_node = Some(arg);
                let arg_ty = self.solve_inferred(arg_ty);
                let other = self.tuple_elements(arg_ty)?;
                let other_fields = self.tuple_fields(arg_ty);
                let recv = self.hoist(recv, recv_ty, span);
                let arg = self.hoist(arg, arg_ty, span);
                let field = |t: &mut Self, of: TExprId, fields: &[SymId], elems: &[TypeId], i: usize| {
                    let of = t.copy_expr(of);
                    t.tuple_element(of, fields, elems, i, span)
                };
                if text == "++" {
                    let mut items: Vec<TExprId> = (0..elems.len()).map(|i| field(self, recv, &fields, &elems, i)).collect();
                    items.extend((0..other.len()).map(|i| field(self, arg, &other_fields, &other, i)));
                    let mut tys = elems.clone();
                    tys.extend(other.iter().copied());
                    let ty = self.tuple_of(&tys);
                    (self.tuple_value(&items), ty)
                } else {
                    let n = elems.len().min(other.len());
                    let mut items = Vec::with_capacity(n);
                    let mut tys = Vec::with_capacity(n);
                    for i in 0..n {
                        let (a, b) = (field(self, recv, &fields, &elems, i), field(self, arg, &other_fields, &other, i));
                        items.push(self.tuple_value(&[a, b]));
                        tys.push(self.tuple_type(&[elems[i], other[i]]));
                    }
                    let ty = self.tuple_of(&tys);
                    (self.tuple_value(&items), ty)
                }
            }
            _ => return None,
        };
        let te = self.wrap_hoisted(mark, result.0);
        // A member that gives the receiver itself (`reverse` of one element) is the receiver's node.
        if self.capturing() && te != recv {
            self.capture_builtin_call(te, name, recv, arg_node.into_iter().collect());
        }
        if !rest.is_empty() {
            self.prog.set_type(te, result.1);
            return Some(self.apply_callee(super::apply::Callee::Value(te, result.1), None, rest.to_vec(), span, None));
        }
        Some((te, result.1))
    }

    /// A literal index past a tuple's elements in an inline body checked at its definition:
    /// `scala.compiletime.error` of scalac's message, a deferred call the expansion reports at
    /// its call, and which an `inline if` that drops the branch, or no call at all, never reaches.
    fn deferred_index_error(&mut self, i: i32, span: Span) -> Option<(TExprId, TypeId)> {
        let compiletime = self.compiletime_pkg()?;
        let name = self.interner.intern("error");
        let sym = match self.pkg_term(compiletime, name)? {
            super::resolve::TermRef::Global(s) | super::resolve::TermRef::ModuleMember(_, s) => s,
            _ => return None,
        };
        let text = self.prog.add_str(&format!("index out of bounds: {}", i));
        let message = self.prog.add(TExpr::Str(text));
        let callee = super::apply::Callee::Method { recv: None, sym, owner_subst: Vec::new(), prefix: None };
        let list = super::apply::ArgList { args: vec![super::apply::ArgSrc::Typed(message, self.b.t_string)], using: false, span };
        Some(self.apply_callee(callee, None, vec![list], span, None))
    }

    /// The tuple of the elements of `recv` at `order`, typed as their elements: `init`,
    /// `reverse`, `take` and `drop` on a tuple type.
    fn tuple_elements_of(&mut self, recv: TExprId, recv_ty: TypeId, fields: &[SymId], elems: &[TypeId], order: &[usize], span: Span) -> (TExprId, TypeId) {
        let t = self.hoist(recv, recv_ty, span);
        let items: Vec<TExprId> = order
            .iter()
            .map(|&i| {
                let t = self.copy_expr(t);
                self.tuple_element(t, fields, elems, i, span)
            })
            .collect();
        let tys: Vec<TypeId> = order.iter().map(|&i| elems[i]).collect();
        let ty = self.tuple_of(&tys);
        (self.tuple_value(&items), ty)
    }

    /// Element `i` of a tuple of the element types `elems`: its field `_i+1` up to 22 elements;
    /// above, `productElement(i)` cast to the element's type, as scalac reads one, since such a
    /// tuple is a TupleXXL at run time where it comes from `Tuple.fromArray` or a mirror.
    pub(super) fn tuple_element(&mut self, recv: TExprId, fields: &[SymId], elems: &[TypeId], i: usize, span: Span) -> TExprId {
        if elems.len() <= 22 {
            return self.prog.add(TExpr::Field(recv, fields[i]));
        }
        let index = self.prog.add(TExpr::Int(i as i32));
        let list = super::apply::ArgList { args: vec![super::apply::ArgSrc::Typed(index, self.b.t_int)], using: false, span };
        let product = self.b.t_product;
        let (te, ty) = self.apply_member(recv, product, crate::names::PRODUCT_ELEMENT, None, vec![list], span, None);
        self.lower_cast(te, ty, elems[i], false, false).0
    }

    /// A tuple of more than 22 elements as scalac makes one: `scala.runtime.TupleXXL.fromIArray`
    /// of the elements in an array of references, the lean std's class on JavaScript and the
    /// interpreter, scala-library's where the jar is the std. None where neither is there.
    pub(super) fn tuple_xxl_value(&mut self, items: &[TExprId], span: Span) -> Option<TExprId> {
        let module = self.tuple_xxl_module()?;
        let recv = self.prog.add(TExpr::Module(module));
        let recv_ty = self.types.class(module, &[]);
        let name = self.interner.intern("fromIArray");
        let (from, _) = self.find_member(recv_ty, name)?;
        let array_ty = self.sig_of(from).clauses.first()?.params.first()?.ty;
        let l = self.prog.list(items);
        let array = self.prog.add(TExpr::ArrayLit(l));
        self.prog.set_type(array, array_ty);
        let list = super::apply::ArgList { args: vec![super::apply::ArgSrc::Typed(array, array_ty)], using: false, span };
        Some(self.apply_member(recv, recv_ty, name, None, vec![list], span, None).0)
    }

    /// The class a value of the class `c` has at run time: TupleXXL for a tuple class past 22
    /// elements, which is a type alone (`classOf`, a `ClassTag`), `c` itself otherwise.
    pub(super) fn runtime_tuple_class(&mut self, c: ClassId) -> ClassId {
        if !self.is_tuple_class(c) || self.syms.class(c).tparams.len() <= 22 {
            return c;
        }
        self.tuple_xxl_module().and_then(|m| self.syms.class(m).companion).unwrap_or(c)
    }

    pub(super) fn tuple_xxl_module(&mut self) -> Option<ClassId> {
        let runtime = self.interner.intern("runtime");
        let p = self.syms.pkg(self.b.scala_pkg).entries.get(&runtime).and_then(|e| e.pkg)?;
        let name = self.interner.intern("TupleXXL");
        let sym = match self.demand_term(p, name) {
            Some(s) => s,
            None => match self.pkg_term(p, name)? {
                super::resolve::TermRef::Global(s) | super::resolve::TermRef::ModuleMember(_, s) => s,
                _ => return None,
            },
        };
        match self.syms.sym(sym).kind {
            SymKind::Object(c) => Some(c),
            _ => None,
        }
    }

    /// The field symbols `_1` to `_N` of a value whose type is a tuple.
    fn tuple_fields(&mut self, t: TypeId) -> Vec<SymId> {
        let t = self.dealias(t);
        match self.types.get(t) {
            Type::Class(c, _) if self.is_tuple_class(c) => self.syms.class(c).ctor_syms.concat(),
            _ => Vec::new(),
        }
    }

    /// Puts a class another worker made under `n` of this worker's table.
    fn note_arity_class(&mut self, n: usize, c: ClassId, table: ArityTable) {
        let table = match table {
            ArityTable::Tuples => &mut self.b.tuples,
            ArityTable::Functions => &mut self.b.functions,
            ArityTable::ContextFunctions => &mut self.b.context_functions,
        };
        if table.len() <= n {
            table.resize(n + 1, None);
        }
        if table[n].is_none() {
            table[n] = Some(c);
        }
    }

    fn register_arity_class(&mut self, n: usize, c: ClassId, table: ArityTable) {
        self.note_arity_class(n, c, table);
        let name = self.syms.class(c).name;
        self.arity_classes.insert(name, c);
        self.syms.pkgs[self.b.scala_pkg.idx()].entries.entry(name).or_default().class = Some(c);
        self.check_class(c);
        if self.loaded.is_some() {
            self.link_library_class(c);
        }
    }
}
