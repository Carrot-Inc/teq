//! Bytecode from the lowered IR (`crate::lir`): each node's code in the order the generator
//! wrote it from the typed IR, so that the class files are the same byte for byte.

use crate::lir::{Arith, Cast, Comparing, ErasedType, Invoke, Kind, LabelId, LirId, List, Relation, SwitchCase, TempId};
use super::classfile::{op, switch_layout, Label, VT};
use super::classes::Declared;
use super::gen::{FinallyFrame, Gen, Local};
use super::lower::holds_recipe_tag;
use super::names::*;
use std::rc::Rc;

/// One call of `makeConcatWithConstants`: `\u{1}` in the recipe stands for an argument,
/// `\u{2}` for a constant that holds one of the two itself.
#[derive(Default)]
struct ConcatCall {
    recipe: String,
    arguments: Vec<ErasedType>,
    constants: Vec<String>,
}

impl ConcatCall {
    fn constant(&mut self, text: String) {
        self.recipe.push('\u{2}');
        self.constants.push(text);
    }

    fn argument(&mut self, t: ErasedType) {
        self.recipe.push('\u{1}');
        self.arguments.push(t);
    }
}

impl<'a> Gen<'a> {
    /// The node's code where code is written after it on its way: an operand's, a statement's, a
    /// value's.
    pub(super) fn write(&mut self, id: LirId) {
        self.write_at(id, true)
    }

    /// The tree of a method's body, its root in return position.
    pub(super) fn write_body(&mut self, root: LirId) {
        self.write_at(root, false)
    }

    /// The node's code; `falls` where code is written after it on its way (master's `Mode::Value`,
    /// dotty's `LoadDestination.FallThrough`), which a conditional's branches, a block's result and a
    /// labelled block's body take from it, and a labelled block among a block's statements (a match's
    /// case in return position).
    fn write_at(&mut self, id: LirId, falls: bool) {
        let node = self.lir.node(id);
        let (kind, ty, of) = (node.kind.clone(), node.ty.clone(), node.of);
        let lines = (self.line_floor, self.line_hold);
        if let Some(place) = of {
            self.mark_line(place);
        }
        self.code.in_node += 1;
        match kind {
            Kind::Int(v) => self.iconst(v),
            Kind::Long(v) => self.lconst(v),
            Kind::Float(v) => self.fconst(v),
            Kind::Double(v) => self.dconst(v),
            Kind::Bool(v) => self.iconst(v as i32),
            Kind::Char(c) => self.iconst(c as i32),
            Kind::Str(text) => self.sconst(&text),
            Kind::Unit => {}
            Kind::Null => {
                self.code.op(op::ACONST_NULL);
                self.code.push(VT::Null);
            }
            Kind::ClassOf(of) => {
                self.push_class(&of);
            }
            Kind::Local(s) => {
                let slot = self.m.locals[&s].slot;
                self.load(slot, &ty);
            }
            Kind::Param(i) => {
                let slot = self.m.params[i as usize].0;
                self.load(slot, &ty);
            }
            Kind::Block { stmts, result } if self.kept_on_stack(stmts, result).is_some() => {
                let (temp, init) = self.kept_on_stack(stmts, result).expect("by the guard");
                self.write(init);
                let reads: Vec<LirId> = (1..stmts.len).map(|i| self.lir.lists[(stmts.start + i) as usize]).chain([result]).collect();
                for (i, &read) in reads.iter().enumerate() {
                    if i + 1 < reads.len() {
                        self.dup();
                    }
                    self.m.stacked = Some(temp);
                    self.write_at(read, falls || i + 1 < reads.len());
                    debug_assert!(self.m.stacked.is_none(), "a temporary kept on the stack and not read first");
                }
            }
            Kind::Block { stmts, result } => {
                let mark = self.code.locals_mark();
                for i in 0..stmts.len {
                    let stmt = self.lir.lists[(stmts.start + i) as usize];
                    let at = falls || !matches!(self.lir.node(stmt).kind, Kind::Labeled { .. });
                    self.write_at(stmt, at);
                }
                match self.lir.node(result).kind.clone() {
                    // A block left by a jump ends its locals before the jump: code after a throw
                    // is verified against the frame the jump stands in.
                    Kind::Break { label, value } => self.write_break(label, value, Some(mark)),
                    _ => {
                        self.write_at(result, falls);
                        self.code.locals_release(mark);
                    }
                }
            }
            Kind::Val { sym, init, cell } => {
                self.write(init);
                let held = self.lir.ty(init).clone();
                let slot = self.store_new(&held);
                let local = match cell {
                    Some(declared) => Local { slot, ty: declared, cell: true },
                    None => Local { slot, ty: held, cell: false },
                };
                self.m.locals.insert(sym, local);
            }
            Kind::Assign { sym, value } => {
                self.write(value);
                let slot = self.m.locals[&sym].slot;
                let held = self.lir.ty(value).clone();
                self.store(slot, &held);
            }
            Kind::Binary { op, left, right } => {
                self.write(left);
                self.write(right);
                self.code.op(arith_instruction(op, &ty));
                self.code.pop();
            }
            Kind::Neg(value) => {
                self.write(value);
                self.code.op(match ty {
                    ErasedType::J => op::LNEG,
                    ErasedType::F => op::FNEG,
                    ErasedType::D => op::DNEG,
                    _ => op::INEG,
                });
                self.code.pop();
                let vt = self.vt(&ty);
                self.code.push(vt);
            }
            Kind::Not(value) => {
                self.write(value);
                self.iconst(1);
                self.code.op(op::IXOR);
                self.code.pop();
            }
            // A condition as a value: one or zero by which way it goes.
            Kind::And(..) | Kind::Or(..) | Kind::Compare { .. } => {
                let no = self.code.new_label();
                let done = self.code.new_label();
                self.write_cond(id, no, false);
                self.iconst(1);
                self.goto(done);
                self.code.pop();
                self.code.bind(no);
                self.iconst(0);
                self.code.bind(done);
            }
            Kind::Helper { method, args } => {
                self.write_arguments(args, None);
                self.invoke_helper(&method);
            }
            // A jump out of a labelled block where the condition holds.
            Kind::If { cond, then, otherwise: None } if matches!(self.lir.node(then).kind, Kind::Break { value: None, .. }) => {
                let Kind::Break { label, .. } = self.lir.node(then).kind else { unreachable!("a break, by the guard") };
                let end = self.m.labels[&label];
                self.write_cond(cond, end, true);
            }
            Kind::If { cond, then, otherwise } => {
                let unless = self.code.new_label();
                self.write_cond(cond, unless, false);
                let mark = self.code.locals_mark();
                match otherwise {
                    None => {
                        self.write(then);
                        self.code.locals_release(mark);
                        self.code.bind(unless);
                    }
                    // Nothing follows a branch with no way past it, nor one that leaves the method or
                    // goes round its loop (`Gen::branch_leaves`): the other starts where it ends, and
                    // the join is where the other's way goes on, if it does.
                    Some(otherwise) => {
                        // Where code follows the conditional, its join is bound whatever its branches do,
                        // as master's `if_` binds it in `Mode::Value`.
                        let then_completes = (falls || self.completes(then)) && !self.branch_leaves(then);
                        let done = self.code.new_label();
                        self.write_at(then, falls);
                        self.code.locals_release(mark);
                        if then_completes {
                            self.leave_to_join(&ty, done);
                        } else if ty != ErasedType::V {
                            self.code.pop();
                        }
                        self.code.bind(unless);
                        self.write_at(otherwise, falls);
                        self.code.locals_release(mark);
                        if falls || then_completes || self.completes(otherwise) {
                            self.code.bind(done);
                        }
                    }
                }
            }
            Kind::While { cond, body } => self.write_while(cond, body),
            Kind::Throw(value) => {
                self.write(value);
                self.code.op(op::ATHROW);
                self.code.pop();
                self.code.end_path();
                // The code after it is dead and still verified: what its place leaves stands there.
                if ty != ErasedType::V {
                    let vt = self.vt(&ty);
                    self.code.push(vt);
                }
            }
            Kind::Return(value) => {
                let frame = self.m.finalizers.last().copied();
                self.write(value);
                self.leave_method(frame, &ty);
            }
            Kind::SelfTailCall { receiver, args } => {
                self.write_self_tail_call(receiver, args);
                if ty != ErasedType::V {
                    let vt = self.vt(&ty);
                    self.code.push(vt);
                }
            }
            Kind::FieldStore { receiver, owner, name, value } => {
                self.write(receiver);
                self.write(value);
                let held = self.lir.ty(value).clone();
                self.putfield(&owner, &name, &held);
            }
            Kind::Module(name) => {
                let own = self.get_module(&name);
                // A type the object's class implements: the frame takes it, and nothing is checked.
                if ty != own {
                    let vt = self.vt(&ty);
                    self.code.retype_top(vt);
                }
            }
            Kind::This => {
                let slot = self.m.this_slot.expect("a lowered `this` has its slot");
                self.load(slot, &ty);
            }
            Kind::Field { receiver, owner, name } => {
                self.write(receiver);
                self.getfield(&owner, &name, &ty);
            }
            Kind::New { touch, ctor, args } => {
                if let Some(module) = touch {
                    let t = self.get_module(&module);
                    self.pop_value(&t);
                }
                self.new_object(&ctor.owner);
                self.write_arguments(args, None);
                self.invoke_mref(&ctor);
            }
            Kind::InitCall { ctor, args } => {
                let this = ErasedType::L(self.this_name.clone());
                self.load(0, &this);
                self.write_arguments(args, None);
                self.invoke_mref(&ctor);
            }
            Kind::Typed(value) => self.write(value),
            Kind::StaticField { owner, name } => self.getstatic(&owner, &name, &ty),
            Kind::StaticFieldStore { owner, name, value } => {
                self.write(value);
                let held = self.lir.ty(value).clone();
                self.putstatic(&owner, &name, &held);
            }
            Kind::StaticFieldSet { owner, name, value } => {
                self.write(value);
                self.dup();
                self.putstatic(&owner, &name, &ty);
            }
            Kind::StaticCall { method, args, order } => {
                self.write_arguments(args, order.as_deref());
                self.invoke_mref(&method);
            }
            Kind::Call { method, at, receiver, args, order } => {
                self.write(receiver);
                self.write_arguments(args, order.as_deref());
                match at {
                    Some((owner, interface)) => {
                        let kind = if interface { Invoke::Interface } else { Invoke::Virtual };
                        self.invoke_desc(kind, &owner, interface, &method.name, &method.desc, method.params.len(), &method.ret);
                    }
                    None => self.invoke_mref(&method),
                }
            }
            Kind::SuperCall { method, receiver, args, order, place } => {
                self.write(receiver);
                self.write_arguments(args, order.as_deref());
                self.super_call_at = place.filter(|&(file, _)| file == self.line_file).map(|(_, end)| end);
                self.invoke_mref(&method);
                self.super_call_at = None;
            }
            Kind::ArrayOf { items } => {
                self.iconst(items.len as i32);
                self.new_array(&ty);
                for i in 0..items.len {
                    let item = self.lir.lists[(items.start + i) as usize];
                    debug_assert_eq!(Self::array_element(&ty).as_ref(), Some(self.lir.ty(item)), "an array literal's item of the array's element type");
                    self.dup();
                    self.iconst(i as i32);
                    self.write(item);
                    self.array_store(&ty);
                }
            }
            Kind::ArrayLoad { array, index } => {
                self.write(array);
                self.write(index);
                let of = self.lir.ty(array).clone();
                self.array_load(&of);
            }
            Kind::ArrayStore { array, index, value } => {
                self.write(array);
                self.write(index);
                self.write(value);
                let of = self.lir.ty(array).clone();
                debug_assert_eq!(Self::array_element(&of).as_ref(), Some(self.lir.ty(value)), "a value stored of the array's element type");
                self.array_store(&of);
            }
            Kind::ArrayLength(array) => {
                self.write(array);
                let of = self.lir.ty(array).clone();
                self.array_length(&of);
            }
            Kind::ArrayClone(array) => {
                self.write(array);
                let of = self.lir.ty(array).clone();
                self.array_clone(&of);
            }
            Kind::NewArray { length } => {
                self.write(length);
                self.new_array(&ty);
            }
            Kind::Cast { value, op } => {
                self.write(value);
                match op {
                    Cast::Erased | Cast::Unchecked => {}
                    Cast::Checked => {
                        let ErasedType::L(class) = &ty else { unreachable!("a class to test") };
                        self.checkcast(class);
                    }
                    Cast::Unbox => self.unbox(&ty),
                    Cast::Nothing => {
                        let exception = "java/lang/ClassCastException";
                        self.new_object(exception);
                        self.sconst("Cannot cast to scala.Nothing");
                        self.invoke(Invoke::Special, exception, false, "<init>", &[ErasedType::L(Rc::from(STRING))], &ErasedType::V);
                        self.code.op(op::ATHROW);
                        self.code.pop();
                        self.code.end_path();
                        // What follows is verified against a null.
                        self.code.push(VT::Null);
                    }
                }
            }
            Kind::StringOf { value } => {
                self.write(value);
                let of = self.lir.ty(value).clone();
                self.string_value_of(&of);
            }
            Kind::Underlying { value } => self.write(value),
            Kind::Concat { parts, calls, operands } => self.write_concat(parts, &calls, &operands),
            Kind::Try { body, catch, finalizer, returns } => {
                self.check_returns(id, returns);
                self.write_try(&ty, body, catch, finalizer, returns)
            }
            Kind::Synchronized { lock, body, returns } => {
                self.check_returns(id, returns);
                self.monitor_block(|g| g.write(lock), ty, returns, |g, _| g.write(body));
            }
            Kind::Adapt { value, from, how } => {
                self.write(value);
                self.write_adaptation(how, &from, &ty);
            }
            Kind::Closure { captures, method, target } => {
                self.write_arguments(captures, None);
                let captured: Vec<ErasedType> = (0..captures.len).map(|i| self.lir.ty(self.lir.lists[(captures.start + i) as usize]).clone()).collect();
                self.lambda_call_site(&target, &captured, &ty, &method);
            }
            Kind::LiftedCall { method, captures, args, order, .. } => {
                self.write_arguments(captures, None);
                self.write_arguments(args, order.as_deref());
                self.invoke_mref(&method);
            }
            Kind::Let { temp, init } => {
                self.write(init);
                let held = self.lir.ty(init).clone();
                let slot = match self.m.reserved.remove(&temp) {
                    Some(reserved) => {
                        let vt = self.vt(&held);
                        self.code.type_local(reserved, vt);
                        self.store(reserved.slot, &held);
                        reserved.slot
                    }
                    None => self.store_new(&held),
                };
                self.m.temps.insert(temp, slot);
            }
            Kind::Reserve { temp, ty: held } => {
                let reserved = self.code.reserve_local(held.wide());
                self.m.temps.insert(temp, reserved.slot);
                self.m.reserved.insert(temp, reserved);
            }
            Kind::LocalAt { sym, temp, ty } => {
                let slot = self.m.temps[&temp];
                self.m.locals.insert(sym, Local { slot, ty, cell: false });
            }
            // The container's `VarHandle` (`Gen::lazy_holder`'s static field, found in the class's `<clinit>`),
            // its signature-polymorphic `compareAndSet` of scalac's descriptor `(Object, Object, Object)Z`.
            Kind::LazyCas { object, owner, field, expected, new } => {
                let handle = ErasedType::L(Rc::from(VAR_HANDLE));
                self.getstatic(&owner, &format!("{}$lzyHandle", field), &handle);
                self.write(object);
                self.write(expected);
                self.write(new);
                let any = ErasedType::object();
                self.invoke(Invoke::Virtual, VAR_HANDLE, false, "compareAndSet", &[any.clone(), any.clone(), any], &ErasedType::Z);
            }
            Kind::SetTemp { temp, value } => {
                self.write(value);
                let held = self.lir.ty(value).clone();
                let slot = self.m.temps[&temp];
                self.store(slot, &held);
            }
            Kind::Temp(temp) if self.m.stacked == Some(temp) => self.m.stacked = None,
            Kind::Temp(temp) => {
                let slot = self.m.temps[&temp];
                self.load(slot, &ty);
            }
            Kind::Is { value, class } => {
                self.write(value);
                self.instance_of(&class);
            }
            Kind::Labeled { label, body } => {
                let end = self.code.new_label();
                self.m.labels.insert(label, end);
                self.write_at(body, falls);
                self.code.bind(end);
            }
            Kind::Break { label, value } => self.write_break(label, value, None),
            Kind::Switch { scrutinee, cases, default } => self.write_switch(&ty, scrutinee, &cases, default, None, falls),
            // The program is refused where the form stands, in the method being written.
            Kind::Refused(refusal) => self.unsupported(&refusal.message()),
        }
        self.code.in_node -= 1;
        (self.line_floor, self.line_hold) = lines;
    }

    /// A loop as dotty's `genWhileDo` writes it (BCodeBodyBuilder.scala 677-705): the head; with no
    /// condition, the body written to the jump back to it (nothing follows); with a unit body, the
    /// condition jumping back to the head where it holds (do-while's shape, `while ({ body; cond }) ()`);
    /// else the condition leaving the loop where it fails, then the body written to the jump back.
    fn write_while(&mut self, cond: Option<LirId>, body: LirId) {
        let head = self.code.new_label();
        self.code.bind(head);
        let Some(cond) = cond else {
            self.write_to(body, head);
            return;
        };
        if matches!(self.lir.node(body).kind, Kind::Unit) {
            self.write_cond(cond, head, true);
            return;
        }
        let exit = self.code.new_label();
        self.write_cond(cond, exit, false);
        self.write_to(body, head);
        self.code.bind(exit);
    }

    /// The node written with a jump to `dest` after it, as dotty's `genLoadTo` writes a tree to a
    /// jump destination (BCodeBodyBuilder.scala 324-528): a conditional's branches each written to
    /// it, the jump where the condition fails standing for a missing `else` (`genLoadIfTo`, 234-270);
    /// a block's result written to it (`genBlockTo`); a labelled block's body written to it, a break
    /// out of the block a jump to it (`genLabeledTo`, 620-637, a match's `genMatchTo` alike); any
    /// other node written, then the jump where its code goes on. The jump out of a labelled block
    /// where a condition holds keeps its own form.
    fn write_to(&mut self, id: LirId, dest: Label) {
        debug_assert!(*self.lir.ty(id) == ErasedType::V, "a value written to a jump");
        let lines = (self.line_floor, self.line_hold);
        if let Some(place) = self.lir.node(id).of {
            self.mark_line(place);
        }
        match self.lir.node(id).kind.clone() {
            Kind::If { cond, then, otherwise } if !(otherwise.is_none() && matches!(self.lir.node(then).kind, Kind::Break { value: None, .. })) => {
                let failure = self.code.new_label();
                self.write_cond(cond, failure, false);
                let mark = self.code.locals_mark();
                self.write_to(then, dest);
                self.code.locals_release(mark);
                self.code.bind(failure);
                match otherwise {
                    Some(otherwise) => {
                        self.write_to(otherwise, dest);
                        self.code.locals_release(mark);
                    }
                    None => self.goto(dest),
                }
            }
            Kind::Block { stmts, result } if self.kept_on_stack(stmts, result).is_none() && !matches!(self.lir.node(result).kind, Kind::Break { .. }) => {
                let mark = self.code.locals_mark();
                for i in 0..stmts.len {
                    let stmt = self.lir.lists[(stmts.start + i) as usize];
                    self.write(stmt);
                }
                self.write_to(result, dest);
                self.code.locals_release(mark);
            }
            Kind::Labeled { label, body } => {
                self.m.labels.insert(label, dest);
                self.write_to(body, dest);
            }
            Kind::Switch { scrutinee, cases, default } => self.write_switch(&ErasedType::V, scrutinee, &cases, default, Some(dest), false),
            _ => {
                self.write(id);
                if self.falls_through(id) {
                    self.goto(dest);
                }
            }
        }
        (self.line_floor, self.line_hold) = lines;
    }

    /// Whether the code of the node goes on after it, as dotty's `genLoadTo` tells by the destination a tree leaves
    /// to (`generatedDest`, BCodeBodyBuilder.scala 324-503): not after a throw, a return, a jump of the tail loop, a
    /// break or a loop with no condition, nor after a block that ends in one or a conditional whose branches both do.
    fn falls_through(&self, id: LirId) -> bool {
        match &self.lir.node(id).kind {
            Kind::Throw(_) | Kind::Return(_) | Kind::SelfTailCall { .. } | Kind::Break { .. } | Kind::While { cond: None, .. } => false,
            Kind::Typed(value) => self.falls_through(*value),
            Kind::Block { stmts, result } => (0..stmts.len).all(|i| self.falls_through(self.lir.lists[(stmts.start + i) as usize])) && self.falls_through(*result),
            Kind::If { then, otherwise: Some(otherwise), .. } => self.falls_through(*then) || self.falls_through(*otherwise),
            Kind::Switch { cases, default, .. } => cases.iter().any(|c| self.falls_through(c.body)) || self.falls_through(*default),
            _ => true,
        }
    }

    /// A `Switch` as dotty's back end writes a match on an `Int` (`genMatchTo`, BCodeBodyBuilder.scala 960-1016 at 3.8.4):
    /// the scrutinee, the switch (`switch_layout`), then each case's body and the default's in their order, each that
    /// falls through followed by the jump to the join, `dest` or the end of the switch (`genLoadTo` to its
    /// `LoadDestination.Jump`, which the last body takes too). A body that is a `Break` without a value out of a
    /// labelled block (not a `try`'s exit) is no code: the switch jumps to the block's end. With no `dest`, each body
    /// is written as the switch is (`falls`), and where code follows the switch its join is bound, as `If`'s is.
    fn write_switch(&mut self, ty: &ErasedType, scrutinee: LirId, cases: &[SwitchCase], default: LirId, dest: Option<Label>, falls: bool) {
        self.write(scrutinee);
        let bodies: Vec<LirId> = cases.iter().map(|c| c.body).chain([default]).collect();
        let mut entries: Vec<(Label, bool)> = Vec::with_capacity(bodies.len());
        for &body in &bodies {
            let out = match self.lir.node(body).kind {
                Kind::Break { label, value: None } if !self.m.try_exits.contains_key(&label) => Some(self.m.labels[&label]),
                _ => None,
            };
            entries.push(match out {
                Some(end) => (end, false),
                None => (self.code.new_label(), true),
            });
        }
        let keys: Vec<(i32, usize)> = cases.iter().enumerate().flat_map(|(i, c)| c.keys.iter().map(move |&k| (k, i))).collect();
        let targets: Vec<Label> = entries.iter().map(|&(label, _)| label).collect();
        let layout = switch_layout(&keys);
        self.code.switch(&layout, &targets[..cases.len()], targets[cases.len()]);
        let join = dest.unwrap_or_else(|| self.code.new_label());
        let mut joined = false;
        let mark = self.code.locals_mark();
        for (&body, &(entry, written)) in bodies.iter().zip(&entries) {
            if !written {
                continue;
            }
            self.code.bind(entry);
            match dest {
                Some(dest) => self.write_to(body, dest),
                None => {
                    self.write_at(body, falls);
                    if self.falls_through(body) {
                        self.goto(join);
                        joined = true;
                        // The value goes with the jump.
                        if *ty != ErasedType::V {
                            self.code.pop();
                        }
                    }
                }
            }
            self.code.locals_release(mark);
        }
        if dest.is_none() && (falls || joined) {
            self.code.bind(join);
        }
    }

    /// The labelled block `label` left with `value`; the locals above `mark` (a block's own, where
    /// the jump is its result) are released before the jump.
    fn write_break(&mut self, label: LabelId, value: Option<LirId>, mark: Option<usize>) {
        if let Some(value) = value {
            self.write(value);
        }
        let exit = self.m.try_exits.get(&label).copied();
        if let Some(mark) = mark {
            self.code.locals_release(mark);
        }
        // A handler of a `try` carries its value to the try's normal exit, a case of a match to the block's end.
        let held = value.map_or(ErasedType::V, |v| self.lir.ty(v).clone());
        let exit = exit.unwrap_or_else(|| self.m.labels[&label]);
        self.leave_to_join(&held, exit);
    }

    /// The return of the value on the stack, `frame` the finalizer or monitor exit that encloses it, where the
    /// writer stood before the value. Off `write`'s frame, which every depth recurses through.
    #[inline(never)]
    fn leave_method(&mut self, frame: Option<FinallyFrame>, ty: &ErasedType) {
        let ret = self.m.ret.clone();
        match frame {
            // Through the enclosing finalizer or monitor exit, as dotty's `genReturn` leaves
            // (BCodeBodyBuilder.scala 649-661): the value kept in the early-return slot, the
            // operands under it dropped, dead as the method leaves, the frame's cleanup taken; the
            // operands dropped stand under the dead code again.
            Some(frame) => {
                match self.m.early_return {
                    Some(slot) if ret != ErasedType::V => self.store(slot, &ret),
                    None if ret != ErasedType::V => self.unsupported("internal error: a return through a finalizer before its slot"),
                    _ => {}
                }
                let dropped = self.drop_stack();
                self.m.exits_taken.push(frame.cleanup);
                self.goto(frame.cleanup);
                for vt in dropped {
                    self.code.push(vt);
                }
            }
            None => self.return_value(&ret),
        }
        // Where a value is wanted, the dead code after it stands over one, as after a jump.
        if *ty != ErasedType::V {
            let vt = self.vt(ty);
            self.code.push(vt);
        }
    }

    /// A `try`'s or a `synchronized` block's recorded `returns` is the tree's answer (`Body::returns_through`).
    fn check_returns(&self, id: LirId, returns: bool) {
        debug_assert!(returns == self.lir.returns_through(id), "the recorded `returns` of {id:?}");
    }

    /// Whether the branch leaves the method or goes round its loop, where a value is wanted: the
    /// loop's jump, a `return` out of the def's own method, alone, as a block's result or in both
    /// branches of a conditional.
    fn branch_leaves(&self, id: LirId) -> bool {
        match &self.lir.node(id).kind {
            Kind::Return(_) | Kind::SelfTailCall { .. } => true,
            Kind::Typed(value) | Kind::Block { result: value, .. } => self.branch_leaves(*value),
            Kind::If { then, otherwise: Some(otherwise), .. } => self.branch_leaves(*then) && self.branch_leaves(*otherwise),
            Kind::Switch { cases, default, .. } => cases.iter().all(|c| self.branch_leaves(c.body)) && self.branch_leaves(*default),
            _ => false,
        }
    }

    /// Whether code is written after the node on its way: not after a return, nor after the
    /// jump back of a self tail call. (After a throw it is, dead and verified.) A block's way goes
    /// through its result and through the labelled blocks among its statements, a labelled block's
    /// through its body: a match in return position, whose cases' bodies return, has no way past
    /// it, the code after a case being the next case's or the `MatchError`'s, which its failures
    /// jump to. After any other statement the block's code is written, though a statement that is
    /// the def's jump back (a match's case at `Unit`) leaves it dead.
    fn completes(&self, id: LirId) -> bool {
        match &self.lir.node(id).kind {
            Kind::Return(_) | Kind::SelfTailCall { .. } => false,
            Kind::Typed(value) => self.completes(*value),
            Kind::If { then, otherwise: Some(otherwise), .. } => self.completes(*then) || self.completes(*otherwise),
            Kind::Switch { cases, default, .. } => cases.iter().any(|c| self.completes(c.body)) || self.completes(*default),
            Kind::Block { stmts, result } => (0..stmts.len).all(|i| {
                let stmt = self.lir.lists[(stmts.start + i) as usize];
                !matches!(self.lir.node(stmt).kind, Kind::Labeled { .. }) || self.completes(stmt)
            }) && self.completes(*result),
            Kind::Labeled { body, .. } => self.completes(*body),
            _ => true,
        }
    }

    /// The statements of an object's constructor tree (`Return(Block(statements, Unit))`) and where its two methods
    /// part them: the number of those up to the superclass's constructor's call, which the `<init>` runs
    /// (`Gen::write_module_part`, `Gen::hold_module_part`).
    pub(super) fn module_statements(&self, root: LirId) -> (Vec<LirId>, usize) {
        let Kind::Return(block) = self.lir.node(root).kind.clone() else { unreachable!("an object's constructor ends in its return") };
        let Kind::Block { stmts, .. } = self.lir.node(block).kind.clone() else { unreachable!("an object's constructor is a block") };
        let all = self.lir.lists[stmts.start as usize..(stmts.start + stmts.len) as usize].to_vec();
        let made = all.iter().position(|&s| matches!(self.lir.node(s).kind, Kind::InitCall { .. })).map_or(0, |i| i + 1);
        (all, made)
    }

    /// A field of the object the `<clinit>` declares where it meets it; the class's plan took it at the part's hold
    /// (`Gen::hold_module_part`).
    fn declare_field(&mut self, f: &Declared) {
        let desc = self.desc_of(&f.ty);
        self.cw.field(super::plan::field_access(f.flags), &f.name, &desc);
    }

    /// The part of an object's constructor tree a method of the class file runs, as dotty's JVM back end places it
    /// (`BCodeSkelBuilder.genPlainClass`, 175-266, `splitAtSuper`): `<init>` the statements up to the superclass's
    /// constructor's call, `<clinit>` the rest, the object's body, once `MODULE$` holds the instance its `this` is read
    /// from (the `<clinit>`'s head, `Gen::hold_module_part`). Scala.js and Scala Native keep the body in the module
    /// class's constructor, as the tree has it.
    /// The fields the object declares (`fields`, by the statement each stands before) are
    /// declared where the `<clinit>` meets them, since a field enters the constant pool as it is
    /// declared.
    pub(super) fn write_module_part(&mut self, root: LirId, init: bool, fields: &[Declared]) {
        let (all, made) = self.module_statements(root);
        debug_assert!(fields.iter().all(|f| f.at >= made), "a field declared in the constructor before the instance is made");
        let part = if init { 0..made } else { made..all.len() };
        // Each part writes of the root its statements and its return.
        self.code.in_node += 1;
        for i in part {
            if !init {
                for f in fields.iter().filter(|f| f.at == i) {
                    self.declare_field(f);
                }
            }
            self.write(all[i]);
        }
        if !init {
            for f in fields.iter().filter(|f| f.at >= all.len()) {
                self.declare_field(f);
            }
        }
        let ret = self.m.ret.clone();
        self.return_value(&ret);
        self.code.in_node -= 1;
    }

    /// The def's call of itself where it loops: its operands into the parameters and `this`, then
    /// the jump back. Where the call stands over operands (in a `return`'s value), they are
    /// dropped, dead as the jump leaves them, and stand under the dead code again.
    fn write_self_tail_call(&mut self, receiver: Option<LirId>, args: List) {
        let tail = self.m.tail.as_ref().expect("a self tail call is lowered where the def loops");
        let params = tail.params.clone();
        let head = self.m.tail_head.expect("the start of a def that loops is bound before its code");
        let dropped = self.drop_stack();
        // The receiver, then the arguments, each into a temporary in the source's order, as
        // dotty's TailRec assigns them (TailRec.scala 350-372); then `this` and the parameters.
        let mark = self.code.locals_mark();
        let receiver = receiver.map(|r| {
            self.write(r);
            let t = self.lir.ty(r).clone();
            (self.store_new(&t), t)
        });
        let mut temps = Vec::new();
        for i in 0..args.len {
            let a = self.lir.lists[(args.start + i) as usize];
            self.write(a);
            let t = self.lir.ty(a).clone();
            temps.push((self.store_new(&t), t));
        }
        if let Some((temp, t)) = receiver {
            self.load(temp, &t);
            let slot = self.m.this_slot.unwrap_or(0);
            self.store(slot, &t);
        }
        for ((slot, t), p) in temps.iter().zip(&params) {
            self.load(*slot, t);
            let target = self.m.locals[p].slot;
            self.store(target, t);
        }
        self.code.locals_release(mark);
        self.goto(head);
        for vt in dropped {
            self.code.push(vt);
        }
    }

    /// Whether a block's temporary is kept on the operand stack, as the class-file writer's choice
    /// of instruction for it (docs/LIR.md, "Composed bodies"): the block's first statement declares
    /// it, and each statement after it and the result reads it as the first value it evaluates, so
    /// that the value made is duplicated for every read but the last and no slot holds it (an enum's
    /// value stored in two fields, `Gen::enum_value_parts`; a template's `dup`, `read_tree`). The
    /// temporary and its value's node.
    fn kept_on_stack(&self, stmts: List, result: LirId) -> Option<(TempId, LirId)> {
        if stmts.len < 2 {
            return None;
        }
        let Kind::Let { temp, init } = self.lir.node(self.lir.lists[stmts.start as usize]).kind else { return None };
        let reads = (1..stmts.len).map(|i| self.lir.lists[(stmts.start + i) as usize]).chain([result]);
        reads.into_iter().all(|id| self.first_read(id) == Some(temp)).then_some((temp, init))
    }

    /// The temporary a node reads as the first value it evaluates, through a store's value, a
    /// call's receiver and an adaptation.
    fn first_read(&self, id: LirId) -> Option<TempId> {
        match self.lir.node(id).kind {
            Kind::Temp(temp) => Some(temp),
            Kind::StaticFieldStore { value, .. } | Kind::Adapt { value, .. } => self.first_read(value),
            Kind::Call { receiver, .. } => self.first_read(receiver),
            _ => None,
        }
    }

    /// A jump to `target` where the condition `id` is `jump_if`, falling through otherwise.
    pub(super) fn write_cond(&mut self, id: LirId, target: Label, jump_if: bool) {
        match self.lir.node(id).kind.clone() {
            Kind::Not(value) => self.write_cond(value, target, !jump_if),
            Kind::And(left, right) => {
                if jump_if {
                    let skip = self.code.new_label();
                    self.write_cond(left, skip, false);
                    self.write_cond(right, target, true);
                    self.code.bind(skip);
                } else {
                    self.write_cond(left, target, false);
                    self.write_cond(right, target, false);
                }
            }
            Kind::Or(left, right) => {
                if jump_if {
                    self.write_cond(left, target, true);
                    self.write_cond(right, target, true);
                } else {
                    let skip = self.code.new_label();
                    self.write_cond(left, skip, true);
                    self.write_cond(right, target, false);
                    self.code.bind(skip);
                }
            }
            Kind::Compare { relation, how, left, right } => self.write_compare(relation, how, left, right, target, jump_if),
            // The statements, then the result's jump, as scalac's `genCond` pushes the decision
            // into a block.
            Kind::Block { stmts, result } => {
                let lines = (self.line_floor, self.line_hold);
                if let Some(place) = self.lir.node(id).of {
                    self.mark_line(place);
                }
                let mark = self.code.locals_mark();
                for i in 0..stmts.len {
                    let stmt = self.lir.lists[(stmts.start + i) as usize];
                    self.write(stmt);
                }
                self.write_cond(result, target, jump_if);
                self.code.locals_release(mark);
                (self.line_floor, self.line_hold) = lines;
            }
            // The loop's jump (`&&`'s or `||`'s right operand under a `return`) leaves: no branch
            // follows it, and the dead code after it is verified against the operands under it.
            Kind::SelfTailCall { receiver, args } => self.write_self_tail_call(receiver, args),
            // A boolean value, tested; a constant left as a condition too (a loop's, a guard's, a
            // right operand, a block's result, a pattern's test that always or never passes), as
            // scalac's back end tests one (`genCond`), at the position of the read it stands for; and
            // an inline method's expansion (`Kind::Typed`, `Gen::expansion_tested`).
            _ => {
                self.write(id);
                self.jump_if(if jump_if { op::IFNE } else { op::IFEQ }, 1, target);
            }
        }
    }

    fn write_compare(&mut self, relation: Relation, how: Comparing, left: LirId, right: LirId, target: Label, jump_if: bool) {
        // A reference compared with the null literal is tested alone, as scalac's
        // `genComparisonOp` (`eq`, `ne`) and `genEqEqPrimitive` (`==` of a class) load only the
        // other side for `genCZJUMP` (`isNull`, `BCodeBodyBuilder.scala` 1582-1586 and 1696-1700
        // at 3.8.4); a null an ascription holds (`Kind::Typed`) is loaded, and an array's `==`
        // loads both (`Comparing::ArrayEquality`).
        if let Comparing::Identity = how {
            let is_null = |g: &Self, id: LirId| matches!(g.lir.node(id).kind, Kind::Null);
            if is_null(self, left) || is_null(self, right) {
                self.write(if is_null(self, left) { right } else { left });
                let equal = (relation == Relation::Eq) == jump_if;
                return self.jump_if(if equal { op::IFNULL } else { op::IFNONNULL }, 1, target);
            }
        }
        // The relation under which the jump is taken.
        let taken = match (relation, jump_if) {
            (r, true) => r,
            (Relation::Lt, false) => Relation::Ge,
            (Relation::Le, false) => Relation::Gt,
            (Relation::Gt, false) => Relation::Le,
            (Relation::Ge, false) => Relation::Lt,
            (Relation::Eq, false) => Relation::Ne,
            (Relation::Ne, false) => Relation::Eq,
        };
        let against_zero = match taken {
            Relation::Lt => op::IFLT,
            Relation::Le => op::IFLE,
            Relation::Gt => op::IFGT,
            Relation::Ge => op::IFGE,
            Relation::Eq => op::IFEQ,
            Relation::Ne => op::IFNE,
        };
        if let Comparing::Sign = how {
            debug_assert!(matches!(self.lir.node(right).kind, Kind::Int(0)), "a sign is compared with zero");
            self.write(left);
            return self.jump_if(against_zero, 1, target);
        }
        self.write(left);
        self.write(right);
        // A NaN operand makes the comparison false: the jump that tests it is not taken, the
        // one that tests its negation is, so NaN's side of `dcmpg` and `dcmpl` follows the
        // comparison as written.
        let nan_high = matches!(relation, Relation::Lt | Relation::Le);
        match how {
            Comparing::Identity | Comparing::ArrayEquality => self.jump_if(if taken == Relation::Eq { op::IF_ACMPEQ } else { op::IF_ACMPNE }, 2, target),
            Comparing::Numbers => match self.lir.ty(left).clone() {
                ErasedType::J | ErasedType::F | ErasedType::D => {
                    let compare = match (self.lir.ty(left), nan_high) {
                        (ErasedType::J, _) => op::LCMP,
                        (ErasedType::F, true) => op::FCMPG,
                        (ErasedType::F, false) => op::FCMPL,
                        (_, true) => op::DCMPG,
                        (_, false) => op::DCMPL,
                    };
                    self.code.op(compare);
                    self.code.popn(2);
                    self.code.push(VT::Int);
                    self.jump_if(against_zero, 1, target);
                }
                _ => {
                    let jump = match taken {
                        Relation::Lt => op::IF_ICMPLT,
                        Relation::Le => op::IF_ICMPLE,
                        Relation::Gt => op::IF_ICMPGT,
                        Relation::Ge => op::IF_ICMPGE,
                        Relation::Eq => op::IF_ICMPEQ,
                        Relation::Ne => op::IF_ICMPNE,
                    };
                    self.jump_if(jump, 2, target);
                }
            },
            Comparing::Ordering(method) => {
                self.invoke_mref(&method);
                self.jump_if(against_zero, 1, target);
            }
            Comparing::Equality(method) => {
                self.invoke_mref(&method);
                self.jump_if(if taken == Relation::Eq { op::IFNE } else { op::IFEQ }, 1, target);
            }
            Comparing::Sign => unreachable!("a sign is written before its operands"),
            Comparing::Units => {
                if taken == Relation::Eq {
                    self.iconst(1);
                    self.jump_if(op::IFNE, 1, target);
                }
            }
        }
    }

    /// A concatenation as scalac writes it: one `invokedynamic` call of
    /// `makeConcatWithConstants` for each of the node's calls, its texts the text of its recipe
    /// and its operands its arguments, each evaluated before the call renders the first (an
    /// operand text a constant of the call where it holds a recipe tag, else loaded); more than
    /// one call's strings are then joined by one more.
    fn write_concat(&mut self, parts: List, calls: &[u32], operands: &[u32]) {
        let string = ErasedType::L(Rc::from(STRING));
        let mark = self.code.locals_mark();
        let mut call = ConcatCall::default();
        let mut next_call = calls.iter().copied().peekable();
        for i in 0..parts.len {
            if next_call.next_if_eq(&i).is_some() {
                self.concat_call(std::mem::take(&mut call));
            }
            let part = self.lir.lists[(parts.start + i) as usize];
            let node = self.lir.node(part);
            let (kind, ty) = (node.kind.clone(), node.ty.clone());
            match kind {
                Kind::Str(text) if operands.contains(&i) && holds_recipe_tag(&text) => call.constant(text.to_string()),
                Kind::Str(text) if operands.contains(&i) => {
                    self.sconst(&text);
                    call.argument(string.clone());
                }
                Kind::Str(text) => call.recipe.push_str(&text),
                _ => {
                    self.write(part);
                    // The parameter that renders the part: a `String`'s own, `Object`'s for any
                    // other reference, a primitive's.
                    call.argument(match ty {
                        ErasedType::L(ref n) if &**n != STRING => ErasedType::object(),
                        t => t,
                    });
                }
            }
        }
        self.concat_call(call);
        if !calls.is_empty() {
            let mut joined = ConcatCall::default();
            for _ in 0..=calls.len() {
                joined.argument(string.clone());
            }
            self.concat_call(joined);
        }
        self.code.locals_release(mark);
    }

    fn concat_call(&mut self, call: ConcatCall) {
        if call.arguments.is_empty() && call.constants.is_empty() {
            return self.sconst(&call.recipe);
        }
        let string = ErasedType::L(Rc::from(STRING));
        let cp = &mut self.cw.cp;
        let factory = cp.method(
            "java/lang/invoke/StringConcatFactory",
            "makeConcatWithConstants",
            "(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/invoke/CallSite;",
        );
        let handle = cp.method_handle(6, factory);
        let mut constants = vec![cp.string(&call.recipe)];
        constants.extend(call.constants.iter().map(|c| cp.string(c)));
        let bootstrap = self.cw.bootstrap_method(handle, &constants);
        let site = self.cw.cp.invoke_dynamic(bootstrap, "makeConcatWithConstants", &method_desc(&call.arguments, &string));
        self.code.op_u16(op::INVOKEDYNAMIC, site);
        self.code.bytes.extend_from_slice(&[0, 0]);
        self.code.popn(call.arguments.len());
        let vt = self.vt(&string);
        self.code.push(vt);
    }

    /// The arguments in the order written; where the method takes them in another (`order`),
    /// into locals and loaded in the method's.
    pub(super) fn write_arguments(&mut self, args: List, order: Option<&[usize]>) {
        for i in 0..args.len {
            let arg = self.lir.lists[(args.start + i) as usize];
            self.write(arg);
        }
        let Some(order) = order else { return };
        debug_assert_eq!(order.len(), args.len as usize);
        let written: Vec<ErasedType> = (0..order.len()).map(|k| self.lir.ty(self.lir.lists[args.start as usize + k]).clone()).collect();
        let mut slots = vec![0u16; written.len()];
        for k in (0..written.len()).rev() {
            slots[k] = self.store_new(&written[k]);
        }
        for &k in order {
            self.load(slots[k], &written[k]);
        }
    }
}

pub(super) fn arith_instruction(op: Arith, ty: &ErasedType) -> u8 {
    use Arith::*;
    match (ty, op) {
        (ErasedType::J, Add) => op::LADD,
        (ErasedType::J, Sub) => op::LSUB,
        (ErasedType::J, Mul) => op::LMUL,
        (ErasedType::J, Div) => op::LDIV,
        (ErasedType::J, Rem) => op::LREM,
        (ErasedType::J, And) => op::LAND,
        (ErasedType::J, Or) => op::LOR,
        (ErasedType::J, Xor) => op::LXOR,
        (ErasedType::J, Shl) => op::LSHL,
        (ErasedType::J, Shr) => op::LSHR,
        (ErasedType::J, Ushr) => op::LUSHR,
        (ErasedType::D, Add) => op::DADD,
        (ErasedType::D, Sub) => op::DSUB,
        (ErasedType::D, Mul) => op::DMUL,
        (ErasedType::D, Div) => op::DDIV,
        (ErasedType::D, _) => op::DREM,
        (ErasedType::F, Add) => op::FADD,
        (ErasedType::F, Sub) => op::FSUB,
        (ErasedType::F, Mul) => op::FMUL,
        (ErasedType::F, Div) => op::FDIV,
        (ErasedType::F, _) => op::FREM,
        (_, Add) => op::IADD,
        (_, Sub) => op::ISUB,
        (_, Mul) => op::IMUL,
        (_, Div) => op::IDIV,
        (_, Rem) => op::IREM,
        (_, And) => op::IAND,
        (_, Or) => op::IOR,
        (_, Xor) => op::IXOR,
        (_, Shl) => op::ISHL,
        (_, Shr) => op::ISHR,
        (_, Ushr) => op::IUSHR,
    }
}
