//! Bytecode from the lowered IR (`crate::lir`): each node's code in the order the generator
//! wrote it from the typed IR, so that the class files are the same byte for byte.

use crate::lir::{ErasedType, Invoke, Kind, LirId, List};
use super::gen::Gen;

impl<'a> Gen<'a> {
    pub(super) fn write(&mut self, id: LirId) {
        let node = self.lir.node(id);
        let (kind, ty, of) = (node.kind.clone(), node.ty.clone(), node.of);
        let lines = (self.line_floor, self.line_hold);
        if let Some(e) = of {
            self.mark_line(e);
        }
        match kind {
            Kind::Legacy(e) => self.expr_legacy(e, &ty),
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
            Kind::Unit => {}
            Kind::Local(s) => {
                let local = self.m.locals[&s].clone();
                self.load(local.slot, &local.ty);
            }
            Kind::Module(name) => {
                self.get_module(&name);
            }
            Kind::This => {
                let slot = self.m.this_slot.expect("a lowered `this` has its slot");
                self.load(slot, &ty);
            }
            Kind::Field { receiver, owner, name } => {
                self.write(receiver);
                self.getfield(&owner, &name, &ty);
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
            Kind::JavaArrayArgument(a) => {
                let ErasedType::L(array) = &ty else { unreachable!("an array's type") };
                self.java_array_argument(a, array);
            }
            Kind::Try { index, body, finalizer } => self.write_try(index, &ty, body, finalizer),
            Kind::Adapt { value, from } => {
                self.write(value);
                self.adapt(&from, &ty);
            }
            Kind::ValueClass { value, from, class, args, null_safe } => {
                self.write(value);
                self.adapt_vc_as(class, args, from, &ty, null_safe);
            }
        }
        (self.line_floor, self.line_hold) = lines;
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
