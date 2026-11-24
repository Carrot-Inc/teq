//! Bytecode from the lowered IR (`crate::lir`): each node's code in the order the generator
//! wrote it from the typed IR, so that the class files are the same byte for byte.

use crate::lir::{Arith, Cast, Comparing, ErasedType, Invoke, Kind, LabelId, LiftedFrom, LirId, List, Relation, TempId, Unlowered};
use super::classfile::{op, Label, VT};
use super::classes::Declared;
use super::gen::{Gen, Lifted, Local, Pending, PendingBody};
use super::names::*;
use std::rc::Rc;

/// scalac's `MaxIndySlots`: a concatenation's call is written before the operand that would
/// bring it to this many operand slots.
const CONCAT_SLOTS: usize = 200;
/// What a constant of the class file holds, in bytes of modified UTF-8.
const RECIPE_BYTES: usize = 65535;

/// One call of `makeConcatWithConstants`: `\u{1}` in the recipe stands for an argument,
/// `\u{2}` for a constant that holds one of the two itself.
#[derive(Default)]
struct ConcatCall {
    recipe: String,
    bytes: usize,
    arguments: Vec<ErasedType>,
    constants: Vec<String>,
    slots: usize,
}

impl ConcatCall {
    fn text(&mut self, text: &str) {
        self.recipe.push_str(text);
        self.bytes += recipe_bytes(text);
    }

    fn constant(&mut self, text: String) {
        self.text("\u{2}");
        self.constants.push(text);
        self.slots += 1;
    }

    fn argument(&mut self, t: ErasedType, slots: usize) {
        self.text("\u{1}");
        self.arguments.push(t);
        self.slots += slots;
    }
}

fn recipe_bytes(text: &str) -> usize {
    text.chars().map(|c| if c == '\0' { 2 } else if c as u32 >= 0x10000 { 6 } else { c.len_utf8() }).sum()
}

impl<'a> Gen<'a> {
    /// A value class's value the old code left on the stack at `t` where the lowering asked another type
    /// (`Gen::expr_marked`: a method's result under an ascription to a wider type, which the static type is): held
    /// in a slot and brought to `want` by `Gen::lower_value_class`'s nodes.
    pub(super) fn write_value_class_left(&mut self, e: crate::tir::TExprId, t: &ErasedType, c: crate::types::ClassId, args: crate::types::TList, null_safe: bool, want: &ErasedType) {
        let slot = self.store_new(t);
        let temp = self.lir.temp();
        self.m.temps.insert(temp, slot);
        let brought = self.lowering(|g| {
            let held = g.lir.push(Kind::Temp(temp), t.clone());
            g.lower_value_class(held, t, c, args, null_safe, want, Some(e))
        });
        self.write(brought);
    }

    /// The node's code where code is written after it on its way: an operand's, a statement's, a
    /// value's the old code writes.
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
        if let Some(e) = of {
            self.mark_line(e);
        }
        self.code.in_node += 1;
        match kind {
            Kind::Legacy(e, why) => {
                self.m.unlowered.push(why);
                self.expr_legacy(e, &ty)
            }
            Kind::Int(v) => self.iconst(v),
            Kind::Long(v) => self.lconst(v),
            Kind::Float(v) => self.fconst(v),
            Kind::Double(v) => self.dconst(v),
            Kind::Bool(v) => self.iconst(v as i32),
            Kind::Char(c) => self.iconst(c as i32),
            Kind::Str(s) => {
                let prog = self.cx.input.prog;
                self.sconst(&prog.strings[s.idx()]);
            }
            Kind::Text(text) => self.sconst(&text),
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
            Kind::LocalFun(sym) => {
                self.m.unlowered.push(Unlowered::LocalDefDeclaration);
                self.lifted_fun(sym);
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
            Kind::And(..) | Kind::Or(..) | Kind::Compare { .. } | Kind::Test { .. } => {
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
                            self.goto(done);
                        }
                        if ty != ErasedType::V {
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
            Kind::While { cond, body } => {
                let head = self.code.new_label();
                let exit = self.code.new_label();
                self.code.bind(head);
                self.write_cond(cond, exit, false);
                let mark = self.code.locals_mark();
                self.write(body);
                self.code.locals_release(mark);
                self.goto(head);
                self.code.bind(exit);
            }
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
                self.write(value);
                let ret = self.m.ret.clone();
                self.return_value(&ret);
            }
            Kind::SelfTailCall { receiver, args } => {
                self.write_self_tail_call(receiver, args);
                if ty != ErasedType::V {
                    let vt = self.vt(&ty);
                    self.code.push(vt);
                }
            }
            Kind::LegacyTail(e, why) => {
                self.m.unlowered.push(why);
                self.tail_legacy(e)
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
                // Transitional: the code a `Legacy` node under the arguments runs decides again
                // how a field of the instance is read before the instance is made
                // (`FieldRoute::Early`), as their lowering decided it.
                let was = std::mem::replace(&mut self.m.before_super, true);
                self.write_arguments(args, None);
                self.m.before_super = was;
                self.invoke_mref(&ctor);
            }
            Kind::Typed(value) => self.write(value),
            Kind::StaticField { owner, name } => self.getstatic(&owner, &name, &ty),
            Kind::StaticFieldStore { owner, name, value } => {
                self.write(value);
                let held = self.lir.ty(value).clone();
                self.putstatic(&owner, &name, &held);
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
            Kind::ArrayOf { elem, items } => {
                self.iconst(items.len as i32);
                self.new_array(&ty);
                for i in 0..items.len {
                    let item = self.lir.lists[(items.start + i) as usize];
                    self.dup();
                    self.iconst(i as i32);
                    self.write(item);
                    self.array_store(&ty, &elem);
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
                let (of, held) = (self.lir.ty(array).clone(), self.lir.ty(value).clone());
                self.array_store(&of, &held);
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
            Kind::Underlying { value, operand } => {
                let outer = self.vc_select.replace(operand);
                self.write(value);
                self.vc_select = outer;
            }
            Kind::Concat { parts } => self.write_concat(parts),
            Kind::Try { body, catch, finalizer } => self.write_try(&ty, body, catch, finalizer),
            Kind::Synchronized { lock, body } => {
                self.monitor_block(|g| g.write(lock), ty, |g, _| g.write(body));
            }
            Kind::Adapt { value, from, how } => {
                self.write(value);
                self.write_adaptation(how, &from, &ty);
            }
            // The lifted method takes its number, and its body its place among the class's
            // methods, where the closure is written.
            Kind::Closure { captures, method, lifted, body } => {
                let name = self.fresh_name("$anonfun");
                let lifted = Rc::new(Lifted::new(name, lifted, body.captures.clone(), body.home, body.module.clone()));
                self.write_arguments(captures, None);
                let (params, code) = body.code.clone().expect("a closure's body");
                self.defer(Pending { lifted: lifted.clone(), body: PendingBody::Lambda(params, code) });
                self.lambda_call_site(&lifted, &ty, &method);
            }
            Kind::LiftedCall { def, from, lifted, captures, args, order } => {
                let named = match from {
                    LiftedFrom::Def => self.lifted_fun(def),
                    LiftedFrom::LazyAccessor | LiftedFrom::LazyInitialiser => self.lazy_method(def, from == LiftedFrom::LazyInitialiser),
                };
                debug_assert!(Rc::ptr_eq(&named.shape, &lifted), "the method named is the one the call was lowered against");
                self.write_arguments(captures, None);
                self.write_arguments(args, order.as_deref());
                let (owner, interface) = match from {
                    LiftedFrom::Def => (self.this_name.clone(), self.is_interface),
                    LiftedFrom::LazyAccessor | LiftedFrom::LazyInitialiser => self.lazy_owner(def),
                };
                self.invoke_desc(Invoke::Static, &owner, interface, &named.name, &named.desc, named.arity(), &named.ret);
            }
            Kind::ReturnKey => self.load_return_key(),
            Kind::LazyKey(holder) => self.load_lazy_key(holder),
            Kind::GivenInstance(class) => {
                let slot = self.m.home_slots.iter().find(|(given, _)| *given == class).expect("an instance the method was given").1;
                self.load(slot, &ty);
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
            // The program is refused where the form stands, in the method being written.
            Kind::Refused(refusal) => self.unsupported(&refusal.message()),
        }
        self.code.in_node -= 1;
        (self.line_floor, self.line_hold) = lines;
    }

    /// The labelled block `label` left with `value`; the locals above `mark` (a block's own, where
    /// the jump is its result) are released before the jump.
    fn write_break(&mut self, label: LabelId, value: Option<LirId>, mark: Option<usize>) {
        if let Some(value) = value {
            self.write(value);
        }
        let exit = self.m.try_exits.get(&label).copied();
        // A case of a `try` leaves its value in the try's result.
        if let (Some((Some(slot), _)), Some(value)) = (exit, value) {
            let held = self.lir.ty(value).clone();
            self.store(slot, &held);
        }
        if let Some(mark) = mark {
            self.code.locals_release(mark);
        }
        match exit {
            Some((_, exit)) => self.goto(exit),
            None => {
                let end = self.m.labels[&label];
                self.goto(end);
                // The value goes with the jump.
                if value.is_some_and(|v| *self.lir.ty(v) != ErasedType::V) {
                    self.code.pop();
                }
            }
        }
    }

    /// Whether the branch leaves the method or goes round its loop, where a value is wanted: the
    /// loop's jump, a `return` out of the def's own method, alone, as a block's result or in both
    /// branches of a conditional (`Gen::leaves`, which a node of typed IR asks).
    fn branch_leaves(&self, id: LirId) -> bool {
        match &self.lir.node(id).kind {
            Kind::SelfTailCall { .. } => true,
            Kind::Legacy(e, _) => self.leaves(*e),
            Kind::Block { result, .. } => self.branch_leaves(*result),
            Kind::If { then, otherwise: Some(otherwise), .. } => self.branch_leaves(*then) && self.branch_leaves(*otherwise),
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
            Kind::Return(_) | Kind::SelfTailCall { .. } | Kind::LegacyTail(..) => false,
            Kind::If { then, otherwise: Some(otherwise), .. } => self.completes(*then) || self.completes(*otherwise),
            Kind::Block { stmts, result } => (0..stmts.len).all(|i| {
                let stmt = self.lir.lists[(stmts.start + i) as usize];
                !matches!(self.lir.node(stmt).kind, Kind::Labeled { .. }) || self.completes(stmt)
            }) && self.completes(*result),
            Kind::Labeled { body, .. } => self.completes(*body),
            _ => true,
        }
    }

    /// The part of an object's constructor tree (`Return(Block(statements, Unit))`) a method of
    /// the class file runs, as dotty's JVM back end places it (`BCodeSkelBuilder.genPlainClass`,
    /// 175-266, `splitAtSuper`): `<init>` the statements up to the superclass's constructor's
    /// call, `<clinit>` the rest, the object's body, once `MODULE$` holds the instance its `this`
    /// is read from (`Gen::begin_module_clinit`). Scala.js and Scala Native keep the body in the
    /// module class's constructor, as the tree has it.
    /// The fields the object declares (`fields`, by the statement each stands before) are
    /// declared where the `<clinit>` meets them, since a field enters the constant pool as it is
    /// declared.
    pub(super) fn write_module_part(&mut self, root: LirId, init: bool, fields: &[Declared]) {
        let Kind::Return(block) = self.lir.node(root).kind.clone() else { unreachable!("an object's constructor ends in its return") };
        let Kind::Block { stmts, .. } = self.lir.node(block).kind.clone() else { unreachable!("an object's constructor is a block") };
        let all = self.lir.lists[stmts.start as usize..(stmts.start + stmts.len) as usize].to_vec();
        let made = all.iter().position(|&s| matches!(self.lir.node(s).kind, Kind::InitCall { .. })).map_or(0, |i| i + 1);
        debug_assert!(fields.iter().all(|f| f.at >= made), "a field declared in the constructor before the instance is made");
        let part = if init { 0..made } else { made..all.len() };
        // Each part writes of the root its statements and its return.
        self.code.in_node += 1;
        for i in part {
            if !init {
                for f in fields.iter().filter(|f| f.at == i) {
                    self.field(f.flags, &f.name, &f.ty);
                }
            }
            self.write(all[i]);
        }
        if !init {
            for f in fields.iter().filter(|f| f.at >= all.len()) {
                self.field(f.flags, &f.name, &f.ty);
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
        let (params, head) = (tail.params.clone(), tail.head);
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
    /// Whether the node is what an inline method's call gave (`Program::is_expansion` of its
    /// expression): dotty's `Inlined`, which the back end loads as a value where a condition is
    /// wanted.
    fn is_expansion_node(&self, id: LirId) -> bool {
        self.lir.node(id).of.is_some_and(|e| self.cx.input.prog.is_expansion(e))
    }

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
            // into a block; a constant result is a value tested, as there, and so is an inline
            // method's expansion, dotty's `Inlined`, which `genCond` does not look into: a block
            // that is one is tested whole, a block whose result is one tests that value.
            Kind::Block { stmts, result } if !self.is_expansion_node(id) => {
                let lines = (self.line_floor, self.line_hold);
                if let Some(e) = self.lir.node(id).of {
                    self.mark_line(e);
                }
                let mark = self.code.locals_mark();
                for i in 0..stmts.len {
                    let stmt = self.lir.lists[(stmts.start + i) as usize];
                    self.write(stmt);
                }
                if matches!(self.lir.node(result).kind, Kind::Bool(_)) || self.is_expansion_node(result) {
                    self.write(result);
                    self.jump_if(if jump_if { op::IFNE } else { op::IFEQ }, 1, target);
                } else {
                    self.write_cond(result, target, jump_if);
                }
                self.code.locals_release(mark);
                (self.line_floor, self.line_hold) = lines;
            }
            Kind::Test { value, test, tested } => {
                let boxed = self.lir.ty(value).clone();
                let mark = self.code.locals_mark();
                self.write(value);
                let slot = self.store_new(&boxed);
                self.test(test, slot, &boxed, Some(tested), target, jump_if);
                self.code.locals_release(mark);
            }
            // The loop's jump (`&&`'s or `||`'s right operand under a `return`) leaves: no branch
            // follows it, and the dead code after it is verified against the operands under it.
            Kind::SelfTailCall { receiver, args } => self.write_self_tail_call(receiver, args),
            // A boolean value, tested; a constant left as a condition too (a loop's, a guard's, a
            // right operand, a pattern's test that always or never passes), as scalac's back end
            // tests one (`genCond`), at the position of the read it stands for.
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
    /// `makeConcatWithConstants`, the constant parts the text of its recipe and the others its
    /// arguments, each evaluated before the call renders the first; a call past the slots one
    /// takes is written before the part that would bring it there, and the calls' strings are
    /// then joined by one more.
    fn write_concat(&mut self, parts: List) {
        let prog = self.cx.input.prog;
        let string = ErasedType::L(Rc::from(STRING));
        let mark = self.code.locals_mark();
        let mut call = ConcatCall::default();
        let mut flushed = 0;
        for i in 0..parts.len {
            let part = self.lir.lists[(parts.start + i) as usize];
            let node = self.lir.node(part);
            let (kind, ty) = (node.kind.clone(), node.ty.clone());
            let text: Option<&str> = match &kind {
                Kind::Str(s) => Some(&prog.strings[s.idx()]),
                Kind::Text(text) => Some(text),
                _ => None,
            };
            // The parameter that renders the part: a `String`'s own, `Object`'s for any other
            // reference, a primitive's.
            let param = match ty {
                ErasedType::L(ref n) if &**n != STRING => ErasedType::object(),
                t => t,
            };
            let slots = if matches!(param, ErasedType::J | ErasedType::D) { 2 } else { 1 };
            if call.slots + slots >= CONCAT_SLOTS {
                self.concat_call(std::mem::take(&mut call));
                flushed += 1;
            }
            match text {
                Some(text) if text.contains(['\u{1}', '\u{2}']) => call.constant(text.to_string()),
                Some(text) if call.bytes + recipe_bytes(text) > RECIPE_BYTES => {
                    self.sconst(text);
                    call.argument(string.clone(), 1);
                }
                Some(text) => call.text(text),
                None => {
                    self.write(part);
                    call.argument(param, slots);
                }
            }
        }
        self.concat_call(call);
        if flushed > 0 {
            let mut joined = ConcatCall::default();
            for _ in 0..=flushed {
                joined.argument(string.clone(), 1);
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

    pub(super) fn write_each(&mut self, nodes: &[LirId]) {
        for &node in nodes {
            self.write(node);
        }
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
