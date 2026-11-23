//! Pattern matching: the decision structure of the JS backend, a sequence of tests and bindings
//! per case against a scrutinee held in a local, with jumps to the next case in the place of
//! nested `if`s.

use crate::lir::{Cases, Comparing, ErasedType, Invoke, Kind, LabelId, LirId, MethodRef, Relation, TempId};
use super::classfile::*;
use super::gen::{Gen, Mode};
use super::names::*;
use crate::ast::ListRef;
use crate::tir::*;
use crate::types::SymId;
use std::rc::Rc;

impl<'a> Gen<'a> {
    pub fn match_(&mut self, scrut: TExprId, cases: ListRef, mode: Mode) {
        let prog = self.cx.input.prog;
        let outer = self.code.locals_mark();
        // A reference is matched as it comes, without a cast to its static type: the patterns
        // test its class, and an unchecked type argument may have promised too much.
        let t = self.static_type(scrut);
        let t = if t.is_ref() || t == ErasedType::V { ErasedType::object() } else { t };
        // A value class's primitive underlying value stays as it is, and is boxed where a pattern
        // tests it.
        let vc = if t.is_ref() { None } else { self.expr_value_class(scrut).map(|(c, _)| c) };
        self.expr(scrut, &t);
        let slot = self.store_new(&t);
        let vc_slots = self.m.vc_slots.len();
        if let Some(c) = vc {
            self.m.vc_slots.push((slot, c));
        }
        let done = self.code.new_label();
        for case in &prog.cases[cases.range()] {
            let next = self.code.new_label();
            let mark = self.code.locals_mark();
            self.pattern(case.pat, slot, &t, next);
            if let Some(g) = case.guard {
                self.cond(g, next, false);
            }
            self.finish(case.body, mode);
            self.code.locals_release(mark);
            if let Mode::Value(want) = mode {
                self.goto(done);
                if *want != ErasedType::V {
                    self.code.pop();
                }
            }
            self.code.bind(next);
        }
        self.match_error(slot, &t);
        self.m.vc_slots.truncate(vc_slots);
        if let Mode::Value(want) = mode {
            if *want != ErasedType::V {
                let vt = self.vt(want);
                self.code.push(vt);
            }
            self.code.bind(done);
        }
        self.code.locals_release(outer);
    }

    /// Tests the value in `slot` against the pattern and binds its variables; jumps to `fail`
    /// when it does not match.
    pub fn pattern(&mut self, pat: TPatId, slot: u16, ty: &ErasedType, fail: Label) {
        let cx = self.cx;
        let prog = cx.input.prog;
        match prog.pats[pat.idx()] {
            TPat::Wildcard => {}
            TPat::Bind(sym, inner) => {
                if let Some(i) = inner {
                    self.pattern(i, slot, ty, fail);
                }
                self.load(slot, ty);
                let declared = self.adapt_to_sym(ty, sym);
                let cell = self.m.cells.contains(&sym);
                self.declare_local(sym, declared, cell);
            }
            TPat::Test(test, tested, inner) => {
                let (slot, ty) = self.boxed_slot(slot, ty);
                // `case Array(a, b)` takes an array of any kind, as scalac's `isArray(x, 1)` does.
                let tested = (!matches!(prog.pats[inner.idx()], TPat::Seq(..))).then_some(tested);
                self.test(test, slot, &ty, tested, fail, false);
                self.pattern(inner, slot, &ty, fail);
            }
            TPat::Equals(e, strict) => {
                let et = self.static_type(e);
                if strict && ty.is_ref() && et.is_ref() && !et.is_string() && !ty.is_string() {
                    // `case _: x.type`: the identity of a reference; `null` tested alone, as
                    // `Gen::write_compare` tests it.
                    if let TExpr::Null = prog.expr(e) {
                        // The test at the pattern's line, which the null's load marked.
                        self.mark_line(e);
                        self.load(slot, ty);
                        return self.jump_if(op::IFNONNULL, 1, fail);
                    }
                    self.expr(e, &et);
                    self.load(slot, ty);
                    return self.jump_if(op::IF_ACMPNE, 2, fail);
                }
                // A value class's underlying value compares as its box (`pattern == scrutinee`,
                // dotty's `EqualTest`, `PatternMatcher.scala` 811-812, the operand boxed by erasure).
                let vc_slot = self.m.vc_slots.iter().any(|&(s, _)| s == slot);
                // So does a value class's instance as the pattern, against a primitive too (`1 match
                // { case `key` => }` for a `key: V` is `key == 1`, `V`'s box unequal to the number).
                let vc_pattern = self.recorded_type(e).and_then(|rt| self.value_class_of(rt)).is_some();
                if !ty.is_ref() && !vc_slot && !vc_pattern {
                    let et = self.static_type(e);
                    if !et.is_ref() && et != ErasedType::V {
                        use crate::typer::prims::*;
                        let k = match (ty.numeric_rank(), et.numeric_rank()) {
                            (Some(a), Some(b)) if a.max(b) == R_DOUBLE => ErasedType::D,
                            (Some(a), Some(b)) if a.max(b) == R_FLOAT => ErasedType::F,
                            (Some(a), Some(b)) if a.max(b) == R_LONG => ErasedType::J,
                            _ => ErasedType::I,
                        };
                        self.load(slot, ty);
                        self.adapt(ty, &k);
                        self.expr(e, &k);
                        let cmp = match k {
                            ErasedType::J => Some(op::LCMP),
                            ErasedType::F => Some(op::FCMPL),
                            ErasedType::D => Some(op::DCMPL),
                            _ => None,
                        };
                        match cmp {
                            Some(cmp) => {
                                self.code.op(cmp);
                                self.code.popn(2);
                                self.code.push(VT::Int);
                                self.jump_if(op::IFNE, 1, fail);
                            }
                            None => self.jump_if(op::IF_ICMPNE, 2, fail),
                        }
                        return;
                    }
                }
                let object = ErasedType::object();
                self.expr(e, &object);
                self.load_slot_as(slot, ty, &object);
                self.helper_call("equal", 2);
                self.jump_if(op::IFEQ, 1, fail);
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
                            // The field's pattern matches the field, the slot's value as itself, not
                            // the class it is the underlying value of: dotty extracts `x1._1` before
                            // testing it (`matchArgsPlan`, `PatternMatcher.scala` 253-262, 411-414), so
                            // `case C(`key`)` compares the `Int` (811-812).
                            let outer = self.m.vc_slots.clone();
                            self.m.vc_slots.retain(|&(s, _)| s != slot);
                            self.pattern(sub, slot, ty, fail);
                            self.m.vc_slots = outer;
                        }
                        return;
                    }
                }
                if !ty.is_ref() {
                    return self.goto(fail);
                }
                // No instance of a class the program never makes, unless another module may.
                if !cx.input.reach.classes[c.idx()] && !cx.linked_class(c) && !cx.input.open_world {
                    return self.goto(fail);
                }
                let name = self.class_name(c);
                self.load(slot, ty);
                self.instance_of(&name);
                self.jump_if(op::IFEQ, 1, fail);
                let fields = prog.sym_list(fields);
                let subs = &prog.pat_lists[subs.range()];
                if subs.iter().all(|&s| matches!(prog.pats[s.idx()], TPat::Wildcard)) {
                    return;
                }
                let class_type = ErasedType::L(name.clone());
                self.load(slot, ty);
                if *ty != class_type {
                    self.checkcast(&name);
                }
                let cslot = self.store_new(&class_type);
                for (&field, &sub) in fields.iter().zip(subs) {
                    if matches!(prog.pats[sub.idx()], TPat::Wildcard) {
                        continue;
                    }
                    let m = self.mref(field);
                    self.load(cslot, &class_type);
                    self.invoke_mref(&m);
                    let ft = m.ret.clone();
                    let fslot = self.store_new(&ft);
                    self.pattern(sub, fslot, &ft, fail);
                }
            }
            TPat::Unapply(sym, call, inner) => {
                self.load(slot, ty);
                let declared = self.adapt_to_sym(ty, sym);
                let cell = self.m.cells.contains(&sym);
                self.declare_local(sym, declared, cell);
                let result = self.static_type(call);
                self.expr(call, &result);
                let rslot = self.store_new(&result);
                self.pattern(inner, rslot, &result, fail);
            }
            TPat::Alt(alts) => {
                let alts = &prog.pat_lists[alts.range()];
                let matched = self.code.new_label();
                let mark = self.code.locals_mark();
                for (i, &alt) in alts.iter().enumerate() {
                    if i + 1 == alts.len() {
                        self.pattern(alt, slot, ty, fail);
                        self.code.locals_release(mark);
                    } else {
                        let next = self.code.new_label();
                        self.pattern(alt, slot, ty, next);
                        self.code.locals_release(mark);
                        self.goto(matched);
                        self.code.bind(next);
                    }
                }
                self.code.bind(matched);
            }
            TPat::Seq(items, rest) => {
                let items = &prog.pat_lists[items.range()];
                let object = ErasedType::object();
                let array = ErasedType::L(Rc::from(OBJECT_ARRAY));
                self.load(slot, ty);
                self.adapt(ty, &object);
                self.iconst(items.len() as i32);
                self.iconst(rest.is_some() as i32);
                let parts_type = self.helper_call("seqPattern", 3);
                self.adapt(&parts_type, &array);
                let parts = self.store_new(&array);
                self.load(parts, &array);
                self.array_length(&array);
                self.jump_if(op::IFEQ, 1, fail);
                for (i, &sub) in items.iter().chain(rest.iter()).enumerate() {
                    if matches!(prog.pats[sub.idx()], TPat::Wildcard) {
                        continue;
                    }
                    self.load(parts, &array);
                    self.iconst(i as i32 + 1);
                    self.array_load(&array);
                    let eslot = self.store_new(&object);
                    self.pattern(sub, eslot, &object, fail);
                }
            }
        }
    }

    /// A primitive scrutinee is boxed for the tests that look at its class.
    fn boxed_slot(&mut self, slot: u16, ty: &ErasedType) -> (u16, ErasedType) {
        if ty.is_ref() {
            return (slot, ty.clone());
        }
        let object = ErasedType::object();
        self.load_slot_as(slot, ty, &object);
        (self.store_new(&object), object)
    }

    /// Loads the value of `slot`, of type `ty`, as `want`: a value class's underlying value a
    /// match holds (`vc_slots`) as its box.
    pub fn load_slot_as(&mut self, slot: u16, ty: &ErasedType, want: &ErasedType) {
        self.load(slot, ty);
        match self.m.vc_slots.iter().rev().find(|&&(s, _)| s == slot) {
            Some(&(_, c)) if want.is_ref() => {
                self.vc_box(c, ty);
                let boxed = self.class_type(c);
                self.adapt(&boxed, want);
            }
            _ => self.adapt(ty, want),
        }
    }

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
    /// leaving through `fail`: as the lowering makes them (`Gen::lower_pattern`), the slot read as a
    /// temporary and `fail` the end of a labelled block, where the pattern is lowered. Transitional:
    /// the failure stands before the tests (`Gen::block_statements`), which no labelled block
    /// says; it ends when the failure follows the tests, a change of the class files of its own.
    pub(super) fn definition_pattern(&mut self, pat: TPatId, slot: u16, ty: &ErasedType, fail: Label) {
        if !self.pattern_lowers(pat) {
            return self.pattern(pat, slot, ty, fail);
        }
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

    /// Whether a type test is lowered (`Gen::lower_test_on`), read by the lowering and by
    /// `Gen::test`: not a test of null or of its absence, which compares with the constant in
    /// another form (`ifnull`) than a comparison of the program (`Gen::write_compare`), nor an
    /// outer test, whose temporary stays held to the end of a pattern's case; nor a connective
    /// over one. Each waits for a change of the class files of its own.
    pub(super) fn test_lowers(&self, test: TestId) -> bool {
        match self.cx.input.prog.tests[test.idx()] {
            TypeTest::Null | TypeTest::AnyRef | TypeTest::AnyVal | TypeTest::Outer(..) => false,
            TypeTest::Or(a, b) | TypeTest::And(a, b) => self.test_lowers(a) && self.test_lowers(b),
            TypeTest::Proven(inner, _) => self.test_lowers(inner),
            _ => true,
        }
    }

    /// Jumps to `target` when the test of the reference in `slot` comes out as `jump_if`: a test
    /// that is lowered as the lowering makes it, the slot read as a temporary of the lowering.
    pub fn test(&mut self, test: TestId, slot: u16, ty: &ErasedType, tested: Option<crate::types::TypeId>, target: Label, jump_if: bool) {
        if self.test_lowers(test) {
            let temp = self.lir.temp();
            self.m.temps.insert(temp, slot);
            let passes = self.lowering(|g| g.lower_test_on(temp, ty, test, tested));
            return self.write_cond(passes, target, jump_if);
        }
        match self.cx.input.prog.tests[test.idx()] {
            TypeTest::Proven(inner, _) => self.test(inner, slot, ty, tested, target, jump_if),
            TypeTest::Null => {
                self.load(slot, ty);
                self.jump_if(if jump_if { op::IFNULL } else { op::IFNONNULL }, 1, target);
            }
            // Every value is boxed to an object where it is tested; `AnyVal` erases to `Object`,
            // so scalac's test takes every value but `null`.
            TypeTest::AnyRef | TypeTest::AnyVal => {
                self.load(slot, ty);
                self.jump_if(if jump_if { op::IFNONNULL } else { op::IFNULL }, 1, target);
            }
            // The members' tests as `Gen::lower_test_on` says.
            TypeTest::Or(a, b) => {
                let (ta, tb) = self.tested_members(tested, false);
                if jump_if {
                    self.test(a, slot, ty, ta, target, true);
                    self.test(b, slot, ty, tb, target, true);
                } else {
                    let yes = self.code.new_label();
                    self.test(a, slot, ty, ta, yes, true);
                    self.test(b, slot, ty, tb, target, false);
                    self.code.bind(yes);
                }
            }
            TypeTest::And(a, b) => {
                let (ta, tb) = self.tested_members(tested, true);
                if jump_if {
                    let no = self.code.new_label();
                    self.test(a, slot, ty, ta, no, false);
                    self.test(b, slot, ty, tb, target, true);
                    self.code.bind(no);
                } else {
                    self.test(a, slot, ty, ta, target, false);
                    self.test(b, slot, ty, tb, target, false);
                }
            }
            // The enclosing instance, read through the outer accessor of the instance's class
            // (which a test before this one established), is tested in its own slot.
            TypeTest::Outer(accessor, inner) => {
                let m = self.mref(accessor);
                self.load(slot, ty);
                self.checkcast(&m.owner);
                self.invoke_mref(&m);
                let outer_ty = m.ret.clone();
                let outer = self.store_new(&outer_ty);
                self.test(inner, outer, &outer_ty, None, target, jump_if);
            }
            _ => unreachable!("lowered (`Gen::lower_test_on`)"),
        }
    }

    /// The condition that the reference in `temp`, of the type `ty`, passes the test `test` of the
    /// type `tested`, a test that is lowered (`Gen::test_lowers`).
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
            TypeTest::Null | TypeTest::AnyRef | TypeTest::AnyVal | TypeTest::Outer(..) => unreachable!("written by `Gen::test` (`Gen::test_lowers`)"),
        };
        let value = self.lir.push(Kind::Temp(temp), ty.clone());
        self.lir.push(Kind::Is { value, class }, ErasedType::Z)
    }

    /// The value in `temp`, of the type `ty`, as `want`.
    fn lower_temp(&mut self, temp: TempId, ty: &ErasedType, want: &ErasedType) -> LirId {
        let value = self.lir.push(Kind::Temp(temp), ty.clone());
        self.lower_adapt(value, ty, want)
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

    /// Whether a match is lowered (`Gen::lower_match`), read by the lowering and by the
    /// generator of the typed IR: each of its patterns is (`Gen::pattern_lowers`), and its
    /// scrutinee is not a value class's underlying value of another erasure than the class's (an
    /// intersection of two value classes), which `Gen::match_` boxes as the class all the same.
    pub(super) fn match_lowers(&mut self, scrut: TExprId, cases: ListRef) -> bool {
        let prog = self.cx.input.prog;
        if !prog.cases[cases.range()].iter().all(|case| self.pattern_lowers(case.pat)) {
            return false;
        }
        let t = self.static_type(scrut);
        if t.is_ref() || t == ErasedType::V {
            return true;
        }
        match self.expr_value_class(scrut) {
            Some((c, args)) => self.vc_erasure(c, args) == t,
            None => true,
        }
    }

    /// Whether a pattern is lowered (`Gen::lower_pattern`): not `null`, a sequence's pattern or
    /// a type test that is not (`Gen::test_lowers`), whose comparisons with `null` and zero and
    /// whose outer test's temporary wait for a change of the class files.
    pub(super) fn pattern_lowers(&self, pat: TPatId) -> bool {
        let prog = self.cx.input.prog;
        match prog.pats[pat.idx()] {
            TPat::Wildcard => true,
            TPat::Bind(_, inner) => inner.is_none_or(|i| self.pattern_lowers(i)),
            TPat::Test(test, _, inner) => self.test_lowers(test) && self.pattern_lowers(inner),
            TPat::Equals(e, _) => !matches!(prog.expr(e), TExpr::Null),
            TPat::Class(_, _, _, subs) => prog.pat_lists[subs.range()].iter().all(|&s| self.pattern_lowers(s)),
            TPat::Unapply(_, _, inner) => self.pattern_lowers(inner),
            TPat::Alt(alts) => prog.pat_lists[alts.range()].iter().all(|&a| self.pattern_lowers(a)),
            TPat::Seq(..) => false,
        }
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
    pub(super) fn lower_catch(&mut self, cases: ListRef, want: &ErasedType) -> Cases {
        let throwable = ErasedType::L(Rc::from(THROWABLE));
        let caught = self.lir.temp();
        let done = self.lir.label();
        let nodes = self.lower_cases(cases, caught, &throwable, Some((done, want)));
        let exception = self.lir.push(Kind::Temp(caught), throwable);
        let result = self.lir.push(Kind::Throw(exception), ErasedType::V);
        let stmts = self.lir.list(&nodes);
        let cases = self.lir.push(Kind::Block { stmts, result }, ErasedType::V);
        Cases::Lowered { caught, done, cases }
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
            Some(&(class, args)) if want.is_ref() => self.lir.push(Kind::ValueClass { value, from: ty.clone(), class, args, null_safe: false }, want.clone()),
            _ => self.lower_adapt(value, ty, want),
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
                    // `case _: x.type`: the identity of a reference.
                    let expected = self.lower_expr(e, &et);
                    let value = self.lir.push(Kind::Temp(temp), ty.clone());
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
            TPat::Seq(..) => unreachable!("written by `Gen::pattern` (`Gen::pattern_lowers`)"),
        }
    }

    /// The pattern's variable `sym` declared with the value in `temp`, of the type `ty`: brought
    /// to the type declared for it, where that is a value class a generic value arriving as its
    /// box (`Gen::adapt_to_sym`); in a cell where a closure assigns or shares it.
    fn lower_bind(&mut self, sym: SymId, temp: TempId, ty: &ErasedType, out: &mut Vec<LirId>) {
        let declared = self.sym_type(sym);
        let value = self.lir.push(Kind::Temp(temp), ty.clone());
        let info = self.cx.input.syms.sym(sym);
        let init = match info.sig.as_ref().and_then(|sig| self.value_class_type(sig.ret)) {
            Some((class, args)) if !info.by_name => self.lir.push(Kind::ValueClass { value, from: ty.clone(), class, args, null_safe: true }, declared.clone()),
            _ => self.lower_adapt(value, ty, &declared),
        };
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
