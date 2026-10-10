//! Pattern matching: the decision structure of the JS backend, a sequence of tests and bindings
//! per case against a scrutinee held in a local, with jumps to the next case in the place of
//! nested `if`s.

use super::classfile::*;
use super::gen::{Gen, Invoke, Mode};
use super::names::*;
use crate::ast::ListRef;
use crate::tir::*;
use std::rc::Rc;

impl<'a> Gen<'a> {
    pub fn match_(&mut self, scrut: TExprId, cases: ListRef, mode: Mode) {
        let prog = self.cx.input.prog;
        let outer = self.code.locals_mark();
        // A reference is matched as it comes, without a cast to its static type: the patterns
        // test its class, and an unchecked type argument may have promised too much.
        let t = self.static_type(scrut);
        let t = if t.is_ref() || t == JType::V { JType::object() } else { t };
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
                if *want != JType::V {
                    self.code.pop();
                }
            }
            self.code.bind(next);
        }
        self.match_error(slot, &t);
        self.m.vc_slots.truncate(vc_slots);
        if let Mode::Value(want) = mode {
            if *want != JType::V {
                let vt = self.vt(want);
                self.code.push(vt);
            }
            self.code.bind(done);
        }
        self.code.locals_release(outer);
    }

    /// Tests the value in `slot` against the pattern and binds its variables; jumps to `fail`
    /// when it does not match.
    pub fn pattern(&mut self, pat: TPatId, slot: u16, ty: &JType, fail: Label) {
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
                    // `case _: x.type`: the identity of a reference.
                    self.expr(e, &et);
                    self.load(slot, ty);
                    return self.jump_if(op::IF_ACMPNE, 2, fail);
                }
                // A value class's underlying value compares as its box (`pattern == scrutinee`,
                // dotty's `EqualTest`, `PatternMatcher.scala` 811-812, the operand boxed by erasure).
                let vc_slot = self.m.vc_slots.iter().any(|&(s, _)| s == slot);
                if !ty.is_ref() && !vc_slot {
                    let et = self.static_type(e);
                    if !et.is_ref() && et != JType::V {
                        use crate::typer::prims::*;
                        let k = match (ty.numeric_rank(), et.numeric_rank()) {
                            (Some(a), Some(b)) if a.max(b) == R_DOUBLE => JType::D,
                            (Some(a), Some(b)) if a.max(b) == R_FLOAT => JType::F,
                            (Some(a), Some(b)) if a.max(b) == R_LONG => JType::J,
                            _ => JType::I,
                        };
                        self.load(slot, ty);
                        self.adapt(ty, &k);
                        self.expr(e, &k);
                        let cmp = match k {
                            JType::J => Some(op::LCMP),
                            JType::F => Some(op::FCMPL),
                            JType::D => Some(op::DCMPL),
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
                let object = JType::object();
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
                let class_type = JType::L(name.clone());
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
                let object = JType::object();
                let array = JType::L(Rc::from(OBJECT_ARRAY));
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
    fn boxed_slot(&mut self, slot: u16, ty: &JType) -> (u16, JType) {
        if ty.is_ref() {
            return (slot, ty.clone());
        }
        let object = JType::object();
        self.load_slot_as(slot, ty, &object);
        (self.store_new(&object), object)
    }

    /// Loads the value of `slot`, of type `ty`, as `want`: a value class's underlying value a
    /// match holds (`vc_slots`) as its box.
    pub fn load_slot_as(&mut self, slot: u16, ty: &JType, want: &JType) {
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

    /// Jumps to `target` when the test of the reference in `slot` comes out as `jump_if`.
    pub fn test(&mut self, test: TestId, slot: u16, ty: &JType, tested: Option<crate::types::TypeId>, target: Label, jump_if: bool) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let class: Rc<str> = match prog.tests[test.idx()] {
            TypeTest::Always => {
                if jump_if {
                    self.goto(target);
                }
                return;
            }
            // No instance of a class that the program never makes; a jar's classes are made by
            // the jar's code in link mode, a JDK class's (`@jvmClass`, `CharSequence` of a string)
            // by the JDK's.
            TypeTest::Class(c) | TypeTest::Trait(c)
                if !cx.input.reach.classes[c.idx()] && cx.input.java_class(c).is_none() && !cx.linked_class(c) && !cx.input.open_world && cx.jvm_class(c).is_none() =>
            {
                if !jump_if {
                    self.goto(target);
                }
                return;
            }
            // A tuple is a `Product` of a tuple class to the JVM, so scalac tests the family through
            // `Tuples.isInstanceOfTuple` and `isInstanceOfNonEmptyTuple`, which know the classes.
            TypeTest::Class(c) | TypeTest::Trait(c) if matches!(&*self.class_name(c), "scala/Tuple" | "scala/NonEmptyTuple" | "scala/$times$colon") => {
                let method = if &*self.class_name(c) == "scala/Tuple" { "isInstanceOfTuple" } else { "isInstanceOfNonEmptyTuple" };
                let object = JType::object();
                self.load(slot, ty);
                self.adapt(ty, &object);
                self.invoke(Invoke::Static, "scala/runtime/Tuples", false, method, &[object], &JType::Z);
                return self.jump_if(if jump_if { op::IFNE } else { op::IFEQ }, 1, target);
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
            // `ScalaRunTime.isArray(x, 1)`, the lean runtime's outside link mode.
            TypeTest::Array => match tested.map(|t| self.erase(t)) {
                Some(JType::L(n)) if n.starts_with('[') => n,
                _ => {
                    let object = JType::object();
                    self.load(slot, ty);
                    self.adapt(ty, &object);
                    self.iconst(1);
                    self.erased_array_call("isArray", &[object, JType::I], JType::Z);
                    return self.jump_if(if jump_if { op::IFNE } else { op::IFEQ }, 1, target);
                }
            },
            TypeTest::Function(n) => Rc::from(format!("scala/Function{}", n)),
            TypeTest::Null => {
                self.load(slot, ty);
                return self.jump_if(if jump_if { op::IFNULL } else { op::IFNONNULL }, 1, target);
            }
            // Every value is boxed to an object where it is tested.
            TypeTest::AnyRef => {
                self.load(slot, ty);
                return self.jump_if(if jump_if { op::IFNONNULL } else { op::IFNULL }, 1, target);
            }
            // `AnyVal` erases to `Object`, so scalac's test takes every value but `null`.
            TypeTest::AnyVal => {
                self.load(slot, ty);
                return self.jump_if(if jump_if { op::IFNONNULL } else { op::IFNULL }, 1, target);
            }
            TypeTest::Value(v) => {
                let object = JType::object();
                let vt = self.static_type(v);
                self.expr(v, &object);
                self.load(slot, ty);
                // A literal type compares the value, a singleton type `x.type` the identity.
                if vt.is_ref() && !vt.is_string() {
                    return self.jump_if(if jump_if { op::IF_ACMPEQ } else { op::IF_ACMPNE }, 2, target);
                }
                self.helper_call("equal", 2);
                return self.jump_if(if jump_if { op::IFNE } else { op::IFEQ }, 1, target);
            }
            TypeTest::Or(a, b) => {
                if jump_if {
                    self.test(a, slot, ty, tested, target, true);
                    self.test(b, slot, ty, tested, target, true);
                } else {
                    let yes = self.code.new_label();
                    self.test(a, slot, ty, tested, yes, true);
                    self.test(b, slot, ty, tested, target, false);
                    self.code.bind(yes);
                }
                return;
            }
            TypeTest::And(a, b) => {
                if jump_if {
                    let no = self.code.new_label();
                    self.test(a, slot, ty, tested, no, false);
                    self.test(b, slot, ty, tested, target, true);
                    self.code.bind(no);
                } else {
                    self.test(a, slot, ty, tested, target, false);
                    self.test(b, slot, ty, tested, target, false);
                }
                return;
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
                return self.test(inner, outer, &outer_ty, None, target, jump_if);
            }
        };
        self.load(slot, ty);
        self.instance_of(&class);
        self.jump_if(if jump_if { op::IFNE } else { op::IFEQ }, 1, target);
    }
}
