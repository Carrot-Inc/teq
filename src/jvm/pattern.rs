//! Pattern matching: the decision structure of the JS backend, a sequence of tests and bindings
//! per case against a scrutinee held in a local, with jumps to the next case in the place of
//! nested `if`s.

use crate::lir::{Catch, Comparing, ErasedType, Invoke, Kind, LabelId, LirId, MethodRef, Relation, TempId};
use super::classfile::*;
use super::gen::Gen;
use super::names::*;
use crate::ast::ListRef;
use crate::tir::*;
use crate::types::SymId;
use std::rc::Rc;

impl<'a> Gen<'a> {
    /// The two members of the tested type where it is a union (`inter`: an intersection) by its form
    /// (`Gen::tested_form`); the type itself for both sides otherwise.
    fn tested_members(&mut self, tested: Option<crate::types::TypeId>, inter: bool) -> (Option<crate::types::TypeId>, Option<crate::types::TypeId>) {
        let Some(t) = tested else { return (None, None) };
        let form = self.tested_form(t);
        match (self.cx.input.types.get(form), inter) {
            (crate::types::Type::Union(a, b, _), false) | (crate::types::Type::Inter(a, b), true) => (Some(a), Some(b)),
            _ => (tested, tested),
        }
    }

    /// The tests and the bindings of a pattern definition's pattern over the value in `slot`,
    /// leaving through `fail`, as the lowering makes them (`Gen::lower_pattern`), the slot read as a
    /// temporary and `fail` the end of a labelled block. Transitional:
    /// the failure stands before the tests (`Gen::block_statements`), which no labelled block
    /// says; it ends when the failure follows the tests, a change of the class files of its own.
    pub(super) fn definition_pattern(&mut self, pat: TPatId, slot: u16, ty: &ErasedType, fail: Label) {
        let temp = self.lir.temp();
        self.m.temps.insert(temp, slot);
        let label = self.lir.label();
        self.m.labels.insert(label, fail);
        let nodes = self.lowering(|g| {
            let mut nodes = Vec::new();
            g.lower_pattern(pat, temp, ty, label, &mut nodes);
            nodes
        });
        for node in nodes {
            self.write(node);
        }
    }

    /// Jumps to `target` when the test of the reference in `slot` comes out as `jump_if`: the
    /// test as the lowering makes it (`Gen::lower_test_on`), the slot read as a temporary of the
    /// lowering.
    pub fn test(&mut self, test: TestId, slot: u16, ty: &ErasedType, tested: Option<crate::types::TypeId>, target: Label, jump_if: bool) {
        let temp = self.lir.temp();
        self.m.temps.insert(temp, slot);
        let passes = self.lowering(|g| g.lower_test_on(temp, ty, test, tested));
        self.write_cond(passes, target, jump_if)
    }

    /// The condition that the reference in `temp`, of the type `ty`, passes the test `test` of the
    /// type `tested`.
    pub(super) fn lower_test_on(&mut self, temp: TempId, ty: &ErasedType, test: TestId, tested: Option<crate::types::TypeId>) -> LirId {
        let cx = self.cx;
        let prog = cx.input.prog;
        let object = ErasedType::object();
        let class: Rc<str> = match prog.tests[test.idx()] {
            // A class test the operand's type proves is the class test still, as scalac's erasure
            // writes it (`TypeTestsCasts` deciding on the erased class).
            TypeTest::Proven(inner, _) => return self.lower_test_on(temp, ty, inner, tested),
            TypeTest::Always => return self.lir.push(Kind::Bool(true), ErasedType::Z),
            // No instance of a class that the program never makes; a jar's classes are made by
            // the jar's code in link mode, a JDK class's (`@jvmClass`, `CharSequence` of a string)
            // by the JDK's.
            TypeTest::Class(c) | TypeTest::Trait(c)
                if !cx.input.reach.classes[c.idx()] && cx.input.java_class(c).is_none() && !cx.linked_class(c) && !cx.input.open_world && cx.jvm_class(c).is_none() =>
            {
                return self.lir.push(Kind::Bool(false), ErasedType::Z);
            }
            // A tuple is a `Product` of a tuple class to the JVM, so scalac tests the family through
            // `Tuples.isInstanceOfTuple` and `isInstanceOfNonEmptyTuple`, which know the classes.
            TypeTest::Class(c) | TypeTest::Trait(c) if matches!(&*self.class_name(c), "scala/Tuple" | "scala/NonEmptyTuple" | "scala/$times$colon") => {
                let method = if &*self.class_name(c) == "scala/Tuple" { "isInstanceOfTuple" } else { "isInstanceOfNonEmptyTuple" };
                let value = self.lower_temp(temp, ty, &object);
                return self.lower_runtime_test("scala/runtime/Tuples", method, &[value]);
            }
            TypeTest::Class(c) | TypeTest::Trait(c) => self.class_name(c),
            TypeTest::Number => Rc::from("java/lang/Double"),
            TypeTest::Int => Rc::from("java/lang/Integer"),
            TypeTest::Long => Rc::from("java/lang/Long"),
            TypeTest::Byte => Rc::from("java/lang/Byte"),
            TypeTest::Short => Rc::from("java/lang/Short"),
            TypeTest::Float => Rc::from("java/lang/Float"),
            TypeTest::Str => Rc::from(STRING),
            TypeTest::Char => Rc::from("java/lang/Character"),
            TypeTest::Bool => Rc::from("java/lang/Boolean"),
            TypeTest::Unit => Rc::from(BOXED_UNIT),
            // A JVM array: its class where the type says which, else scalac's
            // `ScalaRunTime.isArray(x, ndims)` of a generic element's layers (`Gen::array_test_of`), the lean
            // runtime's outside link mode.
            TypeTest::Array => match tested.map_or(Err(1), |t| self.array_test_of(t)) {
                Ok(n) => n,
                Err(layers) => {
                    let value = self.lower_temp(temp, ty, &object);
                    let layers = self.lir.push(Kind::Int(layers), ErasedType::I);
                    return self.lower_runtime_test(SCALA_RUNTIME, "isArray", &[value, layers]);
                }
            },
            TypeTest::Function(n) => Rc::from(format!("scala/Function{}", n)),
            // A literal type compares the value, a singleton type `x.type` the identity.
            TypeTest::Value(v) => {
                let vt = self.static_type(v);
                let expected = self.lower_expr(v, &object);
                if vt.is_ref() && !vt.is_string() {
                    let value = self.lir.push(Kind::Temp(temp), ty.clone());
                    return self.lir.push(Kind::Compare { relation: Relation::Eq, how: Comparing::Identity, left: expected, right: value }, ErasedType::Z);
                }
                let value = self.lower_temp(temp, ty, &object);
                return self.lower_helper_test("equal", &[expected, value]);
            }
            // The test of each member of a union, of both of an intersection (`TypeTestsCasts.transformTypeTest`'s
            // `OrType` and `AndType`): each side with its own member of the tested type, in the order
            // `Worker::test_for` made the two tests of them; a pair made of another type (a class with its outer
            // test, a bound's) reads the whole type on both sides.
            TypeTest::Or(a, b) | TypeTest::And(a, b) => {
                let and = matches!(prog.tests[test.idx()], TypeTest::And(..));
                let (ta, tb) = self.tested_members(tested, and);
                let left = self.lower_test_on(temp, ty, a, ta);
                let right = self.lower_test_on(temp, ty, b, tb);
                return self.lir.push(if and { Kind::And(left, right) } else { Kind::Or(left, right) }, ErasedType::Z);
            }
            TypeTest::Null => return self.lower_null_test(temp, ty, Relation::Eq),
            // Every value is boxed to an object where it is tested; `AnyVal` erases to `Object`,
            // so scalac's test takes every value but `null`.
            TypeTest::AnyRef | TypeTest::AnyVal => return self.lower_null_test(temp, ty, Relation::Ne),
            // The enclosing instance, read through the outer accessor of the instance's class
            // (which a test before this one established), is tested in a temporary of its own.
            TypeTest::Outer(accessor, inner) => {
                let m = self.mref(accessor);
                let value = self.lir.push(Kind::Temp(temp), ty.clone());
                let instance = self.lir.push(Kind::Cast { value, op: crate::lir::Cast::Checked }, ErasedType::L(m.owner.clone()));
                let args = self.lir.list(&[]);
                let outer_ty = m.ret.clone();
                let read = self.lir.push(Kind::Call { method: m, at: None, receiver: instance, args, order: None }, outer_ty.clone());
                let outer = self.lir.temp();
                let held = self.lir.push(Kind::Let { temp: outer, init: read }, ErasedType::V);
                let result = self.lower_test_on(outer, &outer_ty, inner, None);
                let stmts = self.lir.list(&[held]);
                return self.lir.push(Kind::Block { stmts, result }, ErasedType::Z);
            }
        };
        let value = self.lir.push(Kind::Temp(temp), ty.clone());
        self.lir.push(Kind::Is { value, class }, ErasedType::Z)
    }

    /// The value in `temp`, of the type `ty`, as `want`.
    fn lower_temp(&mut self, temp: TempId, ty: &ErasedType, want: &ErasedType) -> LirId {
        let value = self.lir.push(Kind::Temp(temp), ty.clone());
        self.lower_adapt(value, ty, want)
    }

    /// Whether the reference in `temp` is null (`Relation::Eq`) or not (`Ne`).
    fn lower_null_test(&mut self, temp: TempId, ty: &ErasedType, relation: Relation) -> LirId {
        let value = self.lir.push(Kind::Temp(temp), ty.clone());
        let null = self.lir.push(Kind::Null, ErasedType::object());
        self.lir.push(Kind::Compare { relation, how: Comparing::Identity, left: value, right: null }, ErasedType::Z)
    }

    /// A static test of the runtime's class `owner`, its result a boolean.
    fn lower_runtime_test(&mut self, owner: &str, name: &str, args: &[LirId]) -> LirId {
        let params: Vec<ErasedType> = args.iter().map(|&a| self.lir.ty(a).clone()).collect();
        let desc = method_desc(&params, &ErasedType::Z);
        let method = Rc::new(MethodRef { owner: Rc::from(owner), owner_is_interface: false, name: name.to_string(), params, ret: ErasedType::Z, desc, kind: Invoke::Static });
        let args = self.lir.list(args);
        self.lir.push(Kind::StaticCall { method, args, order: None }, ErasedType::Z)
    }

    /// A test by the runtime's helper `name` over the values (`Gen::lower_helper`).
    fn lower_helper_test(&mut self, name: &str, values: &[LirId]) -> LirId {
        self.lower_helper(name, values, ErasedType::Z)
    }

    /// The runtime's helper `name` applied to the values; where the build's runtime has none,
    /// reported, and false or zero by the type `ty` its result would have.
    pub(super) fn lower_helper(&mut self, name: &str, values: &[LirId], ty: ErasedType) -> LirId {
        let Some(&helper) = self.cx.helpers.defs.get(name) else {
            self.unsupported(&format!("the runtime helper {} is missing", name));
            return match ty {
                ErasedType::Z => self.lir.push(Kind::Bool(false), ErasedType::Z),
                _ => self.lir.push(Kind::Int(0), ErasedType::I),
            };
        };
        let method = self.mref(helper);
        let ret = method.ret.clone();
        let args = self.lir.list(values);
        self.lir.push(Kind::Helper { method, args }, ret)
    }

    /// `scrut match { cases }` at the type `want`, or in return position where there is none:
    /// the scrutinee in a temporary; the cases (`Gen::lower_cases`) out of a labelled block that
    /// a case's `Break` leaves with the body's value, or with their bodies in return position;
    /// after the last, a `MatchError` of the scrutinee. A reference is matched as it comes,
    /// without a cast to its static type: the patterns test its class, and an unchecked type
    /// argument may have promised too much. A value class's primitive underlying value stays as
    /// it is, and is boxed where a pattern tests it (`Gen::vc_temps`).
    pub(super) fn lower_match(&mut self, scrut: TExprId, cases: ListRef, want: Option<&ErasedType>) -> LirId {
        let t = self.static_type(scrut);
        let t = if t.is_ref() || t == ErasedType::V { ErasedType::object() } else { t };
        let vc = if t.is_ref() { None } else { self.expr_value_class(scrut) };
        let init = self.lower_expr(scrut, &t);
        let s = self.lir.temp();
        let held = self.lir.push(Kind::Let { temp: s, init }, ErasedType::V);
        if let Some(vc) = vc {
            self.vc_temps.insert(s, vc);
        }
        let done = self.lir.label();
        let nodes = self.lower_cases(cases, s, &t, want.map(|want| (done, want)));
        let ty = want.cloned().unwrap_or(ErasedType::V);
        let failure = self.lower_match_error(s, &t, &ty);
        self.vc_temps.remove(&s);
        let stmts = self.lir.list(&nodes);
        let body = self.lir.push(Kind::Block { stmts, result: failure }, ty.clone());
        let matched = match want {
            Some(_) => self.lir.push(Kind::Labeled { label: done, body }, ty.clone()),
            None => body,
        };
        let stmts = self.lir.list(&[held]);
        self.lir.push(Kind::Block { stmts, result: matched }, ty)
    }

    /// Each case as a labelled block left where a test fails, holding the case's tests over the
    /// value in `temp`, its bindings, its guard and its body: a `Break` to `done` with the body's
    /// value at the type wanted (none for `Unit`), or with no `done` the body in return position.
    fn lower_cases(&mut self, cases: ListRef, temp: TempId, ty: &ErasedType, done: Option<(LabelId, &ErasedType)>) -> Vec<LirId> {
        let prog = self.cx.input.prog;
        let mut nodes = Vec::new();
        for case in &prog.cases[cases.range()] {
            let next = self.lir.label();
            let mut stmts = Vec::new();
            self.lower_pattern(case.pat, temp, ty, next, &mut stmts);
            if let Some(g) = case.guard {
                let guard = self.lower_cond(g);
                self.lower_fail_unless(guard, next, &mut stmts);
            }
            let result = match done {
                None => self.lower_tail(case.body),
                Some((done, want)) => {
                    let value = self.lower_expr(case.body, want);
                    let value = if *want == ErasedType::V {
                        stmts.push(value);
                        None
                    } else {
                        Some(value)
                    };
                    self.lir.push(Kind::Break { label: done, value }, ErasedType::V)
                }
            };
            let stmts = self.lir.list(&stmts);
            let body = self.lir.push(Kind::Block { stmts, result }, ErasedType::V);
            nodes.push(self.lir.push(Kind::Labeled { label: next, body }, ErasedType::V));
        }
        nodes
    }

    /// A `try`'s cases at the type `want` over the exception it caught; after the last, the
    /// exception thrown again.
    pub(super) fn lower_catch(&mut self, cases: ListRef, want: &ErasedType) -> Catch {
        let throwable = ErasedType::L(Rc::from(THROWABLE));
        let caught = self.lir.temp();
        let done = self.lir.label();
        let nodes = self.lower_cases(cases, caught, &throwable, Some((done, want)));
        let exception = self.lir.push(Kind::Temp(caught), throwable);
        let result = self.lir.push(Kind::Throw(exception), ErasedType::V);
        let stmts = self.lir.list(&nodes);
        let cases = self.lir.push(Kind::Block { stmts, result }, ErasedType::V);
        Catch { caught, done, cases }
    }

    /// `MatchError` of the scrutinee in `temp` thrown, at the type `want`.
    fn lower_match_error(&mut self, temp: TempId, ty: &ErasedType, want: &ErasedType) -> LirId {
        let object = ErasedType::object();
        let scrutinee = self.lower_temp_as(temp, ty, &object);
        let params = vec![object];
        let desc = method_desc(&params, &ErasedType::V);
        let ctor = Rc::new(MethodRef { owner: Rc::from(MATCH_ERROR), owner_is_interface: false, name: "<init>".to_string(), params, ret: ErasedType::V, desc, kind: Invoke::Special });
        let args = self.lir.list(&[scrutinee]);
        let error = self.lir.push(Kind::New { touch: None, ctor, args }, ErasedType::L(Rc::from(MATCH_ERROR)));
        self.lir.push(Kind::Throw(error), want.clone())
    }

    /// The value in `temp`, of the type `ty`, as `want`: a value class's underlying value a match
    /// holds (`Gen::vc_temps`) as its box.
    fn lower_temp_as(&mut self, temp: TempId, ty: &ErasedType, want: &ErasedType) -> LirId {
        let value = self.lir.push(Kind::Temp(temp), ty.clone());
        match self.vc_temps.get(&temp) {
            Some(&(class, args)) if want.is_ref() => self.lower_value_class(value, ty, class, args, false, want, None),
            _ => self.lower_adapt(value, ty, want),
        }
    }

    /// A pattern definition `Gen::lowered_tuple_definition` takes, as scalac writes it (dotty's
    /// `makePatDef`, `MatchableTuple`, `Desugar.scala` 1568-1605 and 1622-1671 at 3.8.4, `val $1 =
    /// init match { case $t @ (x1, .., xn) => $t }` then `val xi = $1._i`): `val $1`'s slot first, the scrutinee
    /// held, a labelled block left where it is null, each element read into the pattern's own
    /// binder, the scrutinee the value of the labelled match, its `MatchError` after; then `$1`,
    /// and each variable read from it.
    pub(super) fn lower_tuple_definition(&mut self, pat: TPatId, init: TExprId, nodes: &mut Vec<LirId>) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let TPat::Class(c, ct, _, subs) = prog.pats[pat.idx()] else { unreachable!("a tuple of variables (`Gen::lowered_tuple_definition`)") };
        let subs = prog.pat_lists[subs.range()].to_vec();
        let name = self.class_name(c);
        let tuple = ErasedType::L(name.clone());
        let elems = self.tuple_definition_elements(ct);
        let held = self.lir.temp();
        nodes.push(self.lir.push(Kind::Reserve { temp: held, ty: tuple.clone() }, ErasedType::V));
        let value = self.lower_expr(init, &tuple);
        let x1 = self.lir.temp();
        nodes.push(self.lir.push(Kind::Let { temp: x1, init: value }, ErasedType::V));
        let fail = self.lir.label();
        let done = self.lir.label();
        let mut tests = Vec::new();
        let scrutinee = self.lir.push(Kind::Temp(x1), tuple.clone());
        let null = self.lir.push(Kind::Null, ErasedType::object());
        let there = self.lir.compare(Relation::Ne, Comparing::Identity, scrutinee, null);
        self.lower_fail_unless(there, fail, &mut tests);
        for (i, &sub) in subs.iter().enumerate() {
            match prog.pats[sub.idx()] {
                TPat::Wildcard => {}
                TPat::Bind(sym, None) => {
                    let read = self.lower_tuple_element(x1, &name, &elems, i, Some(sym));
                    let binder = self.lir.temp();
                    tests.push(self.lir.push(Kind::Let { temp: binder, init: read }, ErasedType::V));
                }
                _ => {
                    // A typed element: held, tested, then its variable.
                    let Some((test, tested, sym)) = super::gen::typed_element(prog, sub) else { unreachable!("a typed element (`Gen::lowered_tuple_definition`)") };
                    let read = self.lower_tuple_element(x1, &name, &elems, i, None);
                    let element = self.lir.temp();
                    tests.push(self.lir.push(Kind::Let { temp: element, init: read }, ErasedType::V));
                    // None where its type proves it, else the test as `Gen::test` writes it
                    // (`AnyVal`: that it is not null, as the scrutinee's).
                    match prog.tests[test.idx()] {
                        TypeTest::Always => {}
                        TypeTest::AnyVal => {
                            let value = self.lir.push(Kind::Temp(element), elems[i].clone());
                            let null = self.lir.push(Kind::Null, ErasedType::object());
                            let there = self.lir.compare(Relation::Ne, Comparing::Identity, value, null);
                            self.lower_fail_unless(there, fail, &mut tests);
                        }
                        _ => {
                            let passes = self.lower_test_on(element, &elems[i], test, Some(tested));
                            self.lower_fail_unless(passes, fail, &mut tests);
                        }
                    }
                    if let Some(sym) = sym {
                        let value = self.lir.push(Kind::Temp(element), elems[i].clone());
                        let init = self.lower_to_sym(value, &elems[i], sym).0;
                        let binder = self.lir.temp();
                        tests.push(self.lir.push(Kind::Let { temp: binder, init }, ErasedType::V));
                    }
                }
            }
        }
        let bound = self.lir.temp();
        let whole = self.lir.push(Kind::Temp(x1), tuple.clone());
        tests.push(self.lir.push(Kind::Let { temp: bound, init: whole }, ErasedType::V));
        let yielded = self.lir.push(Kind::Temp(bound), tuple.clone());
        let leave = self.lir.push(Kind::Break { label: done, value: Some(yielded) }, ErasedType::V);
        let stmts = self.lir.list(&tests);
        let body = self.lir.push(Kind::Block { stmts, result: leave }, ErasedType::V);
        let tested = self.lir.push(Kind::Labeled { label: fail, body }, ErasedType::V);
        let failure = self.lower_match_error(x1, &tuple, &tuple);
        let stmts = self.lir.list(&[tested]);
        let body = self.lir.push(Kind::Block { stmts, result: failure }, tuple.clone());
        let matched = self.lir.push(Kind::Labeled { label: done, body }, tuple.clone());
        nodes.push(self.lir.push(Kind::Let { temp: held, init: matched }, ErasedType::V));
        for (i, &sub) in subs.iter().enumerate() {
            let Some(sym) = super::gen::tuple_variable(prog, sub) else { continue };
            let read = self.lower_tuple_element(held, &name, &elems, i, Some(sym));
            let declared = self.lir.ty(read).clone();
            self.lower_declare(sym, read, declared, nodes);
        }
    }

    /// A pattern definition of the general match `Gen::lowered_variables_definition` takes, as scalac
    /// writes it (dotty's `makePatDef`, `None`, `Desugar.scala` 1590-1671 at 3.8.4): the scrutinee
    /// held, a labelled block left where the pattern does not match (`Gen::lower_pattern`, binding
    /// the variables), its value the case's, its `MatchError` after. Without a variable the match
    /// alone; with one, `val x` is the match, its slot taken first (`Kind::LocalAt`); with more, `val $1` is the tuple
    /// of the variables (`specialised_tuple`'s class, else `TupleN$.apply`), its slot taken first,
    /// and each variable is read from it. A for-comprehension's alias (`alias`) is the general match
    /// over `x$1 @ pat` (`makeFor`'s `makeIdPat`, `Desugar.scala` 2214 at 3.8.4): its first element
    /// the scrutinee itself, read back into a temporary of its own (dotty's `x$1`, which teq's pack
    /// does not read).
    pub(super) fn lower_variables_definition(&mut self, pat: TPatId, init: TExprId, alias: bool, nodes: &mut Vec<LirId>) {
        let vars = self.definition_variables(pat).expect("the variables `Gen::lowered_variables_definition` took");
        let parts: Vec<Option<SymId>> = alias.then_some(None).into_iter().chain(vars.iter().map(|&v| Some(v))).collect();
        let n = parts.len();
        let right = self.static_type(init);
        // A Unit right side is matched as its box.
        let t = if right == ErasedType::V { ErasedType::L(Rc::from(BOXED_UNIT)) } else { right.clone() };
        let elems: Vec<ErasedType> = parts.iter().map(|p| p.map_or_else(|| t.clone(), |v| self.sym_type(v))).collect();
        let ty = match n {
            0 => ErasedType::V,
            1 => elems[0].clone(),
            _ => Self::made_tuple_type(&elems),
        };
        let held = self.lir.temp();
        if n > 0 {
            nodes.push(self.lir.push(Kind::Reserve { temp: held, ty: ty.clone() }, ErasedType::V));
        }
        let value = self.lower_expr(init, &right);
        let value = if right == ErasedType::V { self.lower_adapt(value, &right, &t) } else { value };
        let x1 = self.lir.temp();
        nodes.push(self.lir.push(Kind::Let { temp: x1, init: value }, ErasedType::V));
        let fail = self.lir.label();
        let done = self.lir.label();
        let mut tests = Vec::new();
        self.lower_pattern(pat, x1, &t, fail, &mut tests);
        let mut reads: Vec<LirId> = Vec::with_capacity(n);
        for (p, e) in parts.iter().zip(&elems) {
            reads.push(match p {
                Some(v) => self.lir.push(Kind::Local(*v), e.clone()),
                None => self.lir.push(Kind::Temp(x1), e.clone()),
            });
        }
        let yielded = match n {
            0 => None,
            1 => Some(reads[0]),
            _ => Some(self.lower_made_tuple(&reads, &elems)),
        };
        let leave = self.lir.push(Kind::Break { label: done, value: yielded }, ErasedType::V);
        let stmts = self.lir.list(&tests);
        let body = self.lir.push(Kind::Block { stmts, result: leave }, ErasedType::V);
        let tested = self.lir.push(Kind::Labeled { label: fail, body }, ErasedType::V);
        let failure = self.lower_match_error(x1, &t, &ty);
        let stmts = self.lir.list(&[tested]);
        let body = self.lir.push(Kind::Block { stmts, result: failure }, ty.clone());
        let matched = self.lir.push(Kind::Labeled { label: done, body }, ty.clone());
        match n {
            0 => nodes.push(matched),
            1 => {
                nodes.push(self.lir.push(Kind::Let { temp: held, init: matched }, ErasedType::V));
                if let Some(v) = parts[0] {
                    self.lower_declared.insert(v, (elems[0].clone(), false));
                    nodes.push(self.lir.push(Kind::LocalAt { sym: v, temp: held, ty: elems[0].clone() }, ErasedType::V));
                }
            }
            _ => {
                nodes.push(self.lir.push(Kind::Let { temp: held, init: matched }, ErasedType::V));
                let owner: Rc<str> = Rc::from(format!("scala/Tuple{n}"));
                for (i, p) in parts.iter().enumerate() {
                    let read = self.lower_tuple_element(held, &owner, &elems, i, *p);
                    match *p {
                        Some(v) => {
                            self.lower_declared.insert(v, (elems[i].clone(), false));
                            nodes.push(self.lir.push(Kind::Val { sym: v, init: read, cell: None }, ErasedType::V));
                        }
                        None => {
                            let binder = self.lir.temp();
                            nodes.push(self.lir.push(Kind::Let { temp: binder, init: read }, ErasedType::V));
                        }
                    }
                }
            }
        }
    }

    /// A pattern definition of a tuple of variables past 22 (`MatchableTuple` over a `TupleXXL`), as scalac writes it:
    /// `val $1`'s slot first, the scrutinee held, the pattern's tests and bindings (`Gen::lower_pattern`: the class,
    /// `TupleXXL.unapplySeq`, the sequence's length and elements), the scrutinee the match's value, its `MatchError`
    /// after; then each variable `$1.apply(i)` (`Desugar.scala` 1650-1654 at 3.8.4), which `Tuple.apply`'s inline
    /// expansion makes `$1` copied into a temporary and `productElement(i)` of it.
    pub(super) fn lower_xxl_definition(&mut self, pat: TPatId, init: TExprId, nodes: &mut Vec<LirId>) {
        let prog = self.cx.input.prog;
        let items = self.xxl_definition_items(pat).expect("a tuple pattern past 22 (`Gen::xxl_definition_items`)");
        let t = self.static_type(init);
        let held = self.lir.temp();
        nodes.push(self.lir.push(Kind::Reserve { temp: held, ty: t.clone() }, ErasedType::V));
        let value = self.lower_expr(init, &t);
        let x1 = self.lir.temp();
        nodes.push(self.lir.push(Kind::Let { temp: x1, init: value }, ErasedType::V));
        let fail = self.lir.label();
        let done = self.lir.label();
        let mut tests = Vec::new();
        self.lower_pattern(pat, x1, &t, fail, &mut tests);
        let bound = self.lir.temp();
        let whole = self.lir.push(Kind::Temp(x1), t.clone());
        tests.push(self.lir.push(Kind::Let { temp: bound, init: whole }, ErasedType::V));
        let yielded = self.lir.push(Kind::Temp(bound), t.clone());
        let leave = self.lir.push(Kind::Break { label: done, value: Some(yielded) }, ErasedType::V);
        let stmts = self.lir.list(&tests);
        let body = self.lir.push(Kind::Block { stmts, result: leave }, ErasedType::V);
        let tested = self.lir.push(Kind::Labeled { label: fail, body }, ErasedType::V);
        let failure = self.lower_match_error(x1, &t, &t);
        let stmts = self.lir.list(&[tested]);
        let body = self.lir.push(Kind::Block { stmts, result: failure }, t.clone());
        let matched = self.lir.push(Kind::Labeled { label: done, body }, t.clone());
        nodes.push(self.lir.push(Kind::Let { temp: held, init: matched }, ErasedType::V));
        let owner: Rc<str> = Rc::from("scala/runtime/TupleXXL");
        let xxl = ErasedType::L(owner.clone());
        let object = ErasedType::object();
        let method = Rc::new(MethodRef { owner, owner_is_interface: false, name: "productElement".to_string(), params: vec![ErasedType::I], ret: object.clone(), desc: method_desc(&[ErasedType::I], &object), kind: Invoke::Virtual });
        for (i, &item) in items.iter().enumerate() {
            let Some(sym) = super::gen::tuple_variable(prog, item) else { continue };
            let whole = self.lir.push(Kind::Temp(held), t.clone());
            let whole = self.lower_adapt(whole, &t, &xxl);
            let copy = self.lir.temp();
            nodes.push(self.lir.push(Kind::Let { temp: copy, init: whole }, ErasedType::V));
            let receiver = self.lir.push(Kind::Temp(copy), xxl.clone());
            let index = self.lir.int(i as i32);
            let read = self.lir.call(&method, receiver, &[index]);
            let (init, declared) = self.lower_to_sym(read, &object, sym);
            self.lower_declare(sym, init, declared, nodes);
        }
    }

    /// A pattern definition of `makePatDef`'s `SimpleTuple` the typer left as a statement (a typed one, `val (c, d): T =
    /// (e1, e2)`), as scalac writes it (`Desugar.scala` 1638-1662 at 3.8.4: no match, `val $1: T = rhs`, then
    /// `val c = $1._1`, ...): `val $1`'s slot first, the right side at the tuple's class, each variable read from it.
    pub(super) fn lower_simple_tuple_definition(&mut self, pat: TPatId, init: TExprId, nodes: &mut Vec<LirId>) {
        let prog = self.cx.input.prog;
        let TPat::Class(c, ct, _, subs) = prog.pats[pat.idx()] else { unreachable!("a tuple pattern (`Gen::lowered_simple_tuple_definition`)") };
        let subs = prog.pat_lists[subs.range()].to_vec();
        let name = self.class_name(c);
        let tuple = ErasedType::L(name.clone());
        let elems = self.tuple_definition_elements(ct);
        let held = self.lir.temp();
        nodes.push(self.lir.push(Kind::Reserve { temp: held, ty: tuple.clone() }, ErasedType::V));
        let value = self.lower_expr(init, &tuple);
        nodes.push(self.lir.push(Kind::Let { temp: held, init: value }, ErasedType::V));
        for (i, &sub) in subs.iter().enumerate() {
            let Some(sym) = super::gen::tuple_variable(prog, sub) else { continue };
            let read = self.lower_tuple_element(held, &name, &elems, i, Some(sym));
            let declared = self.lir.ty(read).clone();
            self.lower_declare(sym, read, declared, nodes);
        }
    }

    /// The tuple the general match makes of its variables, as `SpecializeTuples` leaves it:
    /// `new Tuple2$mcII$sp(a, b)`-like where `specialised_tuple` names a class, else
    /// `TupleN$.MODULE$.apply` of the variables as objects.
    fn lower_made_tuple(&mut self, reads: &[LirId], elems: &[ErasedType]) -> LirId {
        let made = Self::made_tuple_type(elems);
        if let Some(class) = super::gen::specialised_tuple(elems) {
            let owner: Rc<str> = Rc::from(class.as_str());
            let params = elems.to_vec();
            let desc = method_desc(&params, &ErasedType::V);
            let ctor = Rc::new(MethodRef { owner, owner_is_interface: false, name: "<init>".to_string(), params, ret: ErasedType::V, desc, kind: Invoke::Special });
            let args = self.lir.list(reads);
            return self.lir.push(Kind::New { touch: None, ctor, args }, made);
        }
        let n = reads.len();
        let object = ErasedType::object();
        let module: Rc<str> = Rc::from(format!("scala/Tuple{n}$"));
        let receiver = self.lir.push(Kind::Module(module.clone()), ErasedType::L(module.clone()));
        let args: Vec<LirId> = reads.iter().zip(elems).map(|(&r, t)| self.lower_adapt(r, t, &object)).collect();
        let params = vec![object; n];
        let desc = method_desc(&params, &made);
        let apply = Rc::new(MethodRef { owner: module, owner_is_interface: false, name: "apply".to_string(), params, ret: made, desc, kind: Invoke::Virtual });
        self.lir.call(&apply, receiver, &args)
    }

    /// The class of the tuple `Gen::lower_made_tuple` makes of variables of the types `elems`.
    fn made_tuple_type(elems: &[ErasedType]) -> ErasedType {
        match super::gen::specialised_tuple(elems) {
            Some(class) => ErasedType::L(Rc::from(class.as_str())),
            None => ErasedType::L(Rc::from(format!("scala/Tuple{}", elems.len()))),
        }
    }

    /// The element `i` of the tuple in `temp` as the variable `sym` takes it (as the element's type
    /// without one), by the accessor `Gen::tuple_accessor` names.
    fn lower_tuple_element(&mut self, temp: TempId, owner: &Rc<str>, elems: &[ErasedType], i: usize, sym: Option<SymId>) -> LirId {
        let tuple = ErasedType::L(owner.clone());
        let value = self.lir.push(Kind::Temp(temp), tuple);
        let (accessor, t) = Self::tuple_accessor(elems, i);
        let method = Rc::new(MethodRef { owner: owner.clone(), owner_is_interface: false, name: accessor, params: Vec::new(), ret: t.clone(), desc: method_desc(&[], &t), kind: Invoke::Virtual });
        let read = self.lir.call(&method, value, &[]);
        match sym {
            Some(sym) => self.lower_to_sym(read, &t, sym).0,
            None => self.lower_adapt(read, &t, &elems[i]),
        }
    }

    /// A jump out of the labelled block `fail` where the condition does not hold.
    fn lower_fail_unless(&mut self, cond: LirId, fail: LabelId, out: &mut Vec<LirId>) {
        let fails = self.lir.push(Kind::Not(cond), ErasedType::Z);
        let leave = self.lir.push(Kind::Break { label: fail, value: None }, ErasedType::V);
        out.push(self.lir.push(Kind::If { cond: fails, then: leave, otherwise: None }, ErasedType::V));
    }

    /// The statements that test the value in `temp`, of the type `ty`, against the pattern and
    /// bind its variables, leaving the labelled block `fail` where it does not match.
    pub(super) fn lower_pattern(&mut self, pat: TPatId, temp: TempId, ty: &ErasedType, fail: LabelId, out: &mut Vec<LirId>) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let object = ErasedType::object();
        match prog.pats[pat.idx()] {
            TPat::Wildcard => {}
            TPat::Bind(sym, inner) => {
                if let Some(i) = inner {
                    self.lower_pattern(i, temp, ty, fail, out);
                }
                self.lower_bind(sym, temp, ty, out);
            }
            TPat::Test(test, tested, inner) => {
                // A primitive is boxed for the tests that look at its class.
                let (temp, ty) = if ty.is_ref() {
                    (temp, ty.clone())
                } else {
                    let init = self.lower_temp_as(temp, ty, &object);
                    let boxed = self.lir.temp();
                    out.push(self.lir.push(Kind::Let { temp: boxed, init }, ErasedType::V));
                    (boxed, object.clone())
                };
                // `case Array(a, b)` takes an array of any kind, as scalac's `isArray(x, 1)` does.
                let tested = (!matches!(prog.pats[inner.idx()], TPat::Seq(..))).then_some(tested);
                let passes = self.lower_test_on(temp, &ty, test, tested);
                self.lower_fail_unless(passes, fail, out);
                self.lower_pattern(inner, temp, &ty, fail, out);
            }
            TPat::Equals(e, strict) => {
                let et = self.static_type(e);
                if strict && ty.is_ref() && et.is_ref() && !et.is_string() && !ty.is_string() {
                    // `case _: x.type`: the identity of a reference; `case null`, the null constant's.
                    let expected = match prog.expr(e) {
                        TExpr::Null => self.lir.push(Kind::Null, ErasedType::object()),
                        _ => self.lower_expr(e, &et),
                    };
                    let value = self.lir.push(Kind::Temp(temp), ty.clone());
                    // `case null`'s test stands at the pattern's line, which the null it drops would have marked
                    // (`Gen::write_compare`): the scrutinee's load takes it.
                    if let TExpr::Null = prog.expr(e) {
                        self.lir.nodes[value.0 as usize].of = Some(e);
                    }
                    let same = self.lir.push(Kind::Compare { relation: Relation::Eq, how: Comparing::Identity, left: expected, right: value }, ErasedType::Z);
                    return self.lower_fail_unless(same, fail, out);
                }
                // A value class's underlying value compares as its box (`pattern == scrutinee`,
                // dotty's `EqualTest`, `PatternMatcher.scala` 811-812, the operand boxed by erasure).
                let vc_temp = self.vc_temps.contains_key(&temp);
                // So does a value class's instance as the pattern, against a primitive too (`1 match
                // { case `key` => }` for a `key: V` is `key == 1`, `V`'s box unequal to the number).
                let vc_pattern = self.recorded_type(e).and_then(|rt| self.value_class_of(rt)).is_some();
                if !ty.is_ref() && !vc_temp && !vc_pattern && !et.is_ref() && et != ErasedType::V {
                    use crate::typer::prims::*;
                    let k = match (ty.numeric_rank(), et.numeric_rank()) {
                        (Some(a), Some(b)) if a.max(b) == R_DOUBLE => ErasedType::D,
                        (Some(a), Some(b)) if a.max(b) == R_FLOAT => ErasedType::F,
                        (Some(a), Some(b)) if a.max(b) == R_LONG => ErasedType::J,
                        _ => ErasedType::I,
                    };
                    let value = self.lir.push(Kind::Temp(temp), ty.clone());
                    let left = self.lower_adapt(value, ty, &k);
                    let right = self.lower_expr(e, &k);
                    let equal = self.lir.push(Kind::Compare { relation: Relation::Eq, how: Comparing::Numbers, left, right }, ErasedType::Z);
                    return self.lower_fail_unless(equal, fail, out);
                }
                let expected = self.lower_expr(e, &object);
                let value = self.lower_temp_as(temp, ty, &object);
                let equal = self.lower_helper_test("equal", &[expected, value]);
                self.lower_fail_unless(equal, fail, out);
            }
            TPat::Class(c, _, fields, subs) => {
                // A value class's pattern on its underlying value, which the typer let through as
                // the class itself: no test, and its one field is the value.
                if cx.input.syms.class(c).value_class {
                    let u = self.vc_underlying(c);
                    let held = if self.is_vc_mark(&u) { self.is_vc_mark(ty) } else { *ty == u };
                    if held {
                        let subs = &prog.pat_lists[subs.range()];
                        if let Some(&sub) = subs.first() {
                            // The field's pattern matches the field, the value as itself, not the
                            // class it is the underlying value of: dotty extracts `x1._1` before
                            // testing it (`matchArgsPlan`, `PatternMatcher.scala` 253-262, 411-414), so
                            // `case C(`key`)` compares the `Int` (811-812).
                            let outer = self.vc_temps.remove(&temp);
                            self.lower_pattern(sub, temp, ty, fail, out);
                            if let Some(outer) = outer {
                                self.vc_temps.insert(temp, outer);
                            }
                        }
                        return;
                    }
                }
                // No instance of a class the program never makes, unless another module may.
                if !ty.is_ref() || !cx.input.reach.classes[c.idx()] && !cx.linked_class(c) && !cx.input.open_world {
                    return out.push(self.lir.push(Kind::Break { label: fail, value: None }, ErasedType::V));
                }
                let name = self.class_name(c);
                let value = self.lir.push(Kind::Temp(temp), ty.clone());
                let is = self.lir.push(Kind::Is { value, class: name.clone() }, ErasedType::Z);
                self.lower_fail_unless(is, fail, out);
                let fields = prog.sym_list(fields);
                let subs = &prog.pat_lists[subs.range()];
                if subs.iter().all(|&s| matches!(prog.pats[s.idx()], TPat::Wildcard)) {
                    return;
                }
                let class_type = ErasedType::L(name);
                let value = self.lir.push(Kind::Temp(temp), ty.clone());
                let init = if *ty != class_type { self.lir.push(Kind::Cast { value, op: crate::lir::Cast::Checked }, class_type.clone()) } else { value };
                let instance = self.lir.temp();
                out.push(self.lir.push(Kind::Let { temp: instance, init }, ErasedType::V));
                for (&field, &sub) in fields.iter().zip(subs) {
                    if matches!(prog.pats[sub.idx()], TPat::Wildcard) {
                        continue;
                    }
                    let m = self.mref(field);
                    let ft = m.ret.clone();
                    let receiver = self.lir.push(Kind::Temp(instance), class_type.clone());
                    let args = self.lir.list(&[]);
                    let read = self.lir.push(Kind::Call { method: m, at: None, receiver, args, order: None }, ft.clone());
                    let held = self.lir.temp();
                    out.push(self.lir.push(Kind::Let { temp: held, init: read }, ErasedType::V));
                    self.lower_pattern(sub, held, &ft, fail, out);
                }
            }
            TPat::Unapply(sym, call, inner) => {
                self.lower_bind(sym, temp, ty, out);
                let result = self.static_type(call);
                let init = self.lower_expr(call, &result);
                let held = self.lir.temp();
                out.push(self.lir.push(Kind::Let { temp: held, init }, ErasedType::V));
                self.lower_pattern(inner, held, &result, fail, out);
            }
            // Each alternative but the last in a labelled block it leaves where it fails, the
            // match of one leaving the whole. None (a macro's `Alternatives(Nil)`; no source writes it) tests
            // nothing, as master writes it: its labelled block alone, whose label master binds all the same.
            TPat::Alt(alts) => {
                let alts = &prog.pat_lists[alts.range()];
                let matched = self.lir.label();
                let Some((&last, rest)) = alts.split_last() else {
                    let result = self.lir.push(Kind::Unit, ErasedType::V);
                    let stmts = self.lir.list(&[]);
                    let body = self.lir.push(Kind::Block { stmts, result }, ErasedType::V);
                    return out.push(self.lir.push(Kind::Labeled { label: matched, body }, ErasedType::V));
                };
                let mut nodes = Vec::new();
                for &alt in rest {
                    let next = self.lir.label();
                    let mut stmts = Vec::new();
                    self.lower_pattern(alt, temp, ty, next, &mut stmts);
                    let result = self.lir.push(Kind::Break { label: matched, value: None }, ErasedType::V);
                    let stmts = self.lir.list(&stmts);
                    let body = self.lir.push(Kind::Block { stmts, result }, ErasedType::V);
                    nodes.push(self.lir.push(Kind::Labeled { label: next, body }, ErasedType::V));
                }
                let mut stmts = Vec::new();
                self.lower_pattern(last, temp, ty, fail, &mut stmts);
                let stmts = self.lir.list(&stmts);
                let result = self.lir.push(Kind::Unit, ErasedType::V);
                let result = self.lir.push(Kind::Block { stmts, result }, ErasedType::V);
                let stmts = self.lir.list(&nodes);
                let body = self.lir.push(Kind::Block { stmts, result }, ErasedType::V);
                out.push(self.lir.push(Kind::Labeled { label: matched, body }, ErasedType::V));
            }
            // The runtime's `seqPattern` gives the elements the pattern names and the rest, behind
            // a first element, or an empty array where the sequence does not fit.
            TPat::Seq(items, rest) => {
                let items = &prog.pat_lists[items.range()];
                let array = ErasedType::L(Rc::from(OBJECT_ARRAY));
                let value = self.lir.push(Kind::Temp(temp), ty.clone());
                let sequence = self.lower_adapt(value, ty, &object);
                let count = self.lir.push(Kind::Int(items.len() as i32), ErasedType::I);
                let has_rest = self.lir.push(Kind::Bool(rest.is_some()), ErasedType::Z);
                let Some(&helper) = cx.helpers.defs.get("seqPattern") else {
                    self.unsupported("the runtime helper seqPattern is missing");
                    return out.push(self.lir.push(Kind::Break { label: fail, value: None }, ErasedType::V));
                };
                let method = self.mref(helper);
                let ret = method.ret.clone();
                let args = self.lir.list(&[sequence, count, has_rest]);
                let parts = self.lir.push(Kind::Helper { method, args }, ret.clone());
                let init = self.lower_adapt(parts, &ret, &array);
                let parts = self.lir.temp();
                out.push(self.lir.push(Kind::Let { temp: parts, init }, ErasedType::V));
                let held = self.lir.push(Kind::Temp(parts), array.clone());
                let length = self.lir.push(Kind::ArrayLength(held), ErasedType::I);
                let zero = self.lir.push(Kind::Int(0), ErasedType::I);
                let empty = self.lir.push(Kind::Compare { relation: Relation::Eq, how: Comparing::Sign, left: length, right: zero }, ErasedType::Z);
                let leave = self.lir.push(Kind::Break { label: fail, value: None }, ErasedType::V);
                out.push(self.lir.push(Kind::If { cond: empty, then: leave, otherwise: None }, ErasedType::V));
                for (i, &sub) in items.iter().chain(rest.iter()).enumerate() {
                    if matches!(prog.pats[sub.idx()], TPat::Wildcard) {
                        continue;
                    }
                    let array_value = self.lir.push(Kind::Temp(parts), array.clone());
                    let index = self.lir.push(Kind::Int(i as i32 + 1), ErasedType::I);
                    let init = self.lir.push(Kind::ArrayLoad { array: array_value, index }, object.clone());
                    let element = self.lir.temp();
                    out.push(self.lir.push(Kind::Let { temp: element, init }, ErasedType::V));
                    self.lower_pattern(sub, element, &object, fail, out);
                }
            }
        }
    }

    /// The pattern's variable `sym` declared with the value in `temp`, of the type `ty`: brought
    /// to the type declared for it, where that is a value class a generic value arriving as its
    /// box (`Gen::lower_to_sym`); in a cell where a closure assigns or shares it.
    fn lower_bind(&mut self, sym: SymId, temp: TempId, ty: &ErasedType, out: &mut Vec<LirId>) {
        let value = self.lir.push(Kind::Temp(temp), ty.clone());
        self.lower_define(sym, value, ty, out);
    }

    /// The variable `sym` declared with `value`, of the type `ty`, as `Gen::lower_bind` declares it.
    fn lower_define(&mut self, sym: SymId, value: LirId, ty: &ErasedType, out: &mut Vec<LirId>) {
        let (init, declared) = self.lower_to_sym(value, ty, sym);
        self.lower_declare(sym, init, declared, out);
    }

    /// The variable `sym` declared with `init`, a value of its declared type `declared`: in a cell
    /// where a closure assigns or shares it.
    fn lower_declare(&mut self, sym: SymId, init: LirId, declared: ErasedType, out: &mut Vec<LirId>) {
        if !self.m.cells.contains(&sym) {
            self.lower_declared.insert(sym, (declared, false));
            return out.push(self.lir.push(Kind::Val { sym, init, cell: None }, ErasedType::V));
        }
        let object = ErasedType::object();
        let content = self.lower_adapt(init, &declared, &object);
        let held = self.lir.temp();
        out.push(self.lir.push(Kind::Let { temp: held, init: content }, ErasedType::V));
        let cell: Rc<str> = Rc::from(OBJECT_REF);
        let value = self.lir.push(Kind::Temp(held), object.clone());
        let args = self.lir.list(&[value]);
        let params = vec![object];
        let desc = method_desc(&params, &ErasedType::V);
        let ctor = Rc::new(MethodRef { owner: cell.clone(), owner_is_interface: false, name: "<init>".to_string(), params, ret: ErasedType::V, desc, kind: Invoke::Special });
        let made = self.lir.push(Kind::New { touch: None, ctor, args }, ErasedType::L(cell));
        self.lower_declared.insert(sym, (declared.clone(), true));
        out.push(self.lir.push(Kind::Val { sym, init: made, cell: Some(declared) }, ErasedType::V));
    }
}
