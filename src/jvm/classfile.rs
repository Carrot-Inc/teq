//! A JVM class file writer: constant pool with deduplication, fields, methods, and a code builder
//! that tracks the types of the locals and of the operand stack as instructions are appended.
//! The stack map frames come from that model: the emitter produces structured code, so the state
//! at every branch target is known when the label is bound or first jumped to, and no dataflow
//! analysis over the bytecode is needed.

use crate::intern::FxMap;

pub const ACC_PUBLIC: u16 = 0x0001;
pub const ACC_PRIVATE: u16 = 0x0002;
pub const ACC_STATIC: u16 = 0x0008;
pub const ACC_FINAL: u16 = 0x0010;
/// The Java releases whose class files the writer makes, scalac 3.8's `-java-output-version`:
/// 17 is the oldest it takes and the default, and the newest here is the newest the suites run.
pub const OUTPUT_VERSIONS: std::ops::RangeInclusive<u32> = 17..=24;
pub const DEFAULT_OUTPUT_VERSION: u32 = 17;

pub fn major_version(output_version: u32) -> u16 {
    (44 + output_version) as u16
}

/// `MODULE$` of a module class, which its `<clinit>` alone stores.
pub const MODULE_FIELD: u16 = ACC_PUBLIC | ACC_STATIC | ACC_FINAL;
pub const ACC_SUPER: u16 = 0x0020;
pub const ACC_BRIDGE: u16 = 0x0040;
/// The bit of `ACC_BRIDGE` on a field.
pub const ACC_VOLATILE: u16 = 0x0040;
pub const ACC_INTERFACE: u16 = 0x0200;
pub const ACC_ABSTRACT: u16 = 0x0400;
pub const ACC_SYNTHETIC: u16 = 0x1000;

#[derive(Clone, PartialEq, Eq, Hash)]
enum Const {
    Class(u16),
    Str(u16),
    Int(i32),
    Float(u32),
    Long(i64),
    Double(u64),
    NameAndType(u16, u16),
    Field(u16, u16),
    Method(u16, u16),
    InterfaceMethod(u16, u16),
    MethodHandle(u8, u16),
    MethodType(u16),
    InvokeDynamic(u16, u16),
}

#[derive(Default)]
pub struct ConstPool {
    bytes: Vec<u8>,
    count: u16,
    utf8: FxMap<Box<str>, u16>,
    consts: FxMap<Const, u16>,
}

impl ConstPool {
    fn new() -> ConstPool {
        ConstPool { bytes: Vec::with_capacity(1024), count: 1, utf8: FxMap::default(), consts: FxMap::default() }
    }

    pub fn utf8(&mut self, s: &str) -> u16 {
        if let Some(&i) = self.utf8.get(s) {
            return i;
        }
        let i = self.count;
        self.count += 1;
        self.bytes.push(1);
        let encoded = modified_utf8(s);
        self.bytes.extend_from_slice(&(encoded.len() as u16).to_be_bytes());
        self.bytes.extend_from_slice(&encoded);
        self.utf8.insert(s.into(), i);
        i
    }

    fn add(&mut self, c: Const) -> u16 {
        if let Some(&i) = self.consts.get(&c) {
            return i;
        }
        let i = self.count;
        let b = &mut self.bytes;
        let mut wide = false;
        match c {
            Const::Class(n) => {
                b.push(7);
                b.extend_from_slice(&n.to_be_bytes());
            }
            Const::Str(n) => {
                b.push(8);
                b.extend_from_slice(&n.to_be_bytes());
            }
            Const::Int(v) => {
                b.push(3);
                b.extend_from_slice(&v.to_be_bytes());
            }
            Const::Float(v) => {
                b.push(4);
                b.extend_from_slice(&v.to_be_bytes());
            }
            Const::Long(v) => {
                b.push(5);
                b.extend_from_slice(&v.to_be_bytes());
                wide = true;
            }
            Const::Double(v) => {
                b.push(6);
                b.extend_from_slice(&v.to_be_bytes());
                wide = true;
            }
            Const::NameAndType(n, d) => {
                b.push(12);
                b.extend_from_slice(&n.to_be_bytes());
                b.extend_from_slice(&d.to_be_bytes());
            }
            Const::Field(..) | Const::Method(..) | Const::InterfaceMethod(..) => unreachable!("member references go through `member`"),
            Const::MethodHandle(kind, r) => {
                b.push(15);
                b.push(kind);
                b.extend_from_slice(&r.to_be_bytes());
            }
            Const::MethodType(d) => {
                b.push(16);
                b.extend_from_slice(&d.to_be_bytes());
            }
            Const::InvokeDynamic(bsm, nt) => {
                b.push(18);
                b.extend_from_slice(&bsm.to_be_bytes());
                b.extend_from_slice(&nt.to_be_bytes());
            }
        }
        self.count += if wide { 2 } else { 1 };
        self.consts.insert(c, i);
        i
    }

    fn member(&mut self, tag: u8, c: Const) -> u16 {
        if let Some(&i) = self.consts.get(&c) {
            return i;
        }
        let (class, nt) = match c {
            Const::Field(a, b) | Const::Method(a, b) | Const::InterfaceMethod(a, b) => (a, b),
            _ => unreachable!(),
        };
        let i = self.count;
        self.count += 1;
        self.bytes.push(tag);
        self.bytes.extend_from_slice(&class.to_be_bytes());
        self.bytes.extend_from_slice(&nt.to_be_bytes());
        self.consts.insert(c, i);
        i
    }

    pub fn class(&mut self, name: &str) -> u16 {
        let n = self.utf8(name);
        self.add(Const::Class(n))
    }
    pub fn string(&mut self, s: &str) -> u16 {
        let n = self.utf8(s);
        self.add(Const::Str(n))
    }
    pub fn int(&mut self, v: i32) -> u16 {
        self.add(Const::Int(v))
    }
    pub fn float(&mut self, v: f32) -> u16 {
        self.add(Const::Float(v.to_bits()))
    }
    pub fn long(&mut self, v: i64) -> u16 {
        self.add(Const::Long(v))
    }
    pub fn double(&mut self, v: f64) -> u16 {
        self.add(Const::Double(v.to_bits()))
    }
    pub fn name_and_type(&mut self, name: &str, desc: &str) -> u16 {
        let n = self.utf8(name);
        let d = self.utf8(desc);
        self.add(Const::NameAndType(n, d))
    }
    pub fn field(&mut self, class: &str, name: &str, desc: &str) -> u16 {
        let c = self.class(class);
        let nt = self.name_and_type(name, desc);
        self.member(9, Const::Field(c, nt))
    }
    pub fn method(&mut self, class: &str, name: &str, desc: &str) -> u16 {
        let c = self.class(class);
        let nt = self.name_and_type(name, desc);
        self.member(10, Const::Method(c, nt))
    }
    pub fn interface_method(&mut self, class: &str, name: &str, desc: &str) -> u16 {
        let c = self.class(class);
        let nt = self.name_and_type(name, desc);
        self.member(11, Const::InterfaceMethod(c, nt))
    }
    pub fn method_handle(&mut self, kind: u8, reference: u16) -> u16 {
        self.add(Const::MethodHandle(kind, reference))
    }
    pub fn method_type(&mut self, desc: &str) -> u16 {
        let d = self.utf8(desc);
        self.add(Const::MethodType(d))
    }
    pub fn invoke_dynamic(&mut self, bootstrap: u16, name: &str, desc: &str) -> u16 {
        let nt = self.name_and_type(name, desc);
        self.add(Const::InvokeDynamic(bootstrap, nt))
    }
}

fn modified_utf8(s: &str) -> Vec<u8> {
    if s.bytes().all(|b| b != 0 && b < 0x80) {
        return s.as_bytes().to_vec();
    }
    let mut out = Vec::with_capacity(s.len() + 8);
    for unit in crate::text::utf16_units(s) {
        match unit {
            0x0001..=0x007f => out.push(unit as u8),
            0 | 0x0080..=0x07ff => {
                out.push(0xc0 | (unit >> 6) as u8);
                out.push(0x80 | (unit & 0x3f) as u8);
            }
            _ => {
                out.push(0xe0 | (unit >> 12) as u8);
                out.push(0x80 | ((unit >> 6) & 0x3f) as u8);
                out.push(0x80 | (unit & 0x3f) as u8);
            }
        }
    }
    out
}

/// A verification type of the stack map. `Top2` is the second slot of a long or double local.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VT {
    Top,
    Top2,
    Int,
    Float,
    Long,
    Double,
    Null,
    UninitThis,
    /// A class by its constant pool index.
    Object(u16),
    /// The result of the `new` instruction at this offset, before its constructor ran.
    Uninit(u16),
}

impl VT {
    fn wide(self) -> bool {
        matches!(self, VT::Long | VT::Double)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Label(u32);

struct LabelInfo {
    offset: Option<u32>,
    /// How many local slots existed when the label was made: what is declared later belongs to
    /// one of the paths that meet at the label, not to the code after it.
    locals_len: usize,
    /// The state at the label, taken from the first jump to it or from where it is bound.
    frame: Option<(Vec<VT>, Vec<VT>)>,
}

struct Frame {
    offset: u32,
    locals: Vec<VT>,
    stack: Vec<VT>,
}

pub struct Handler {
    pub start: Label,
    pub end: Label,
    pub handler: Label,
    /// Constant pool index of the caught class; 0 catches everything.
    pub catch_type: u16,
}

pub struct Code {
    /// The locals on entry, which the first frame is relative to.
    initial_locals: Vec<VT>,
    pub bytes: Vec<u8>,
    pub stack: Vec<VT>,
    pub locals: Vec<VT>,
    depth: u16,
    max_stack: u16,
    max_locals: u16,
    labels: Vec<LabelInfo>,
    fixups: Vec<(u32, u32, Label)>,
    frames: Vec<Frame>,
    handlers: Vec<Handler>,
    lines: Vec<(u16, u16)>,
    /// False after an unconditional transfer until a label is bound.
    pub reachable: bool,
    /// Every jump takes a 32-bit offset: a conditional one as its inverse over a `goto_w`. For
    /// the methods whose code is too long for 16-bit offsets.
    pub wide: bool,
    /// Set by `finish` when a 16-bit offset did not reach its target.
    pub overflow: bool,
    /// Set when a pop met an empty model: the method's code is wrong and is reported, not
    /// written.
    pub underflow: bool,
}

impl Code {
    pub fn new(locals: Vec<VT>) -> Code {
        let max_locals = locals.len() as u16;
        Code {
            initial_locals: locals.clone(),
            bytes: Vec::with_capacity(256),
            stack: Vec::new(),
            locals,
            depth: 0,
            max_stack: 0,
            max_locals,
            labels: Vec::new(),
            fixups: Vec::new(),
            frames: Vec::new(),
            handlers: Vec::new(),
            lines: Vec::new(),
            reachable: true,
            wide: false,
            overflow: false,
            underflow: false,
        }
    }

    pub fn pc(&self) -> u32 {
        self.bytes.len() as u32
    }

    pub fn line(&mut self, line: u32) {
        let pc = self.pc() as u16;
        let line = line.min(u16::MAX as u32) as u16;
        match self.lines.last_mut() {
            Some(last) if last.1 == line => {}
            Some(last) if last.0 == pc => last.1 = line,
            _ => self.lines.push((pc, line)),
        }
    }

    // ---- the model ----

    pub fn push(&mut self, t: VT) {
        self.depth += if t.wide() { 2 } else { 1 };
        self.max_stack = self.max_stack.max(self.depth);
        self.stack.push(t);
    }

    /// The source line the code last recorded, for a report.
    pub fn last_line(&self) -> Option<u16> {
        self.lines.last().map(|&(_, l)| l)
    }

    pub fn pop(&mut self) -> VT {
        let Some(t) = self.stack.pop() else {
            self.underflow = true;
            return VT::Null;
        };
        self.depth -= if t.wide() { 2 } else { 1 };
        t
    }

    pub fn popn(&mut self, n: usize) {
        for _ in 0..n {
            self.pop();
        }
    }

    /// Gives the top of the stack the type the emitter takes it for from here on; the verifier
    /// sees a type assignable to it.
    pub fn retype_top(&mut self, t: VT) {
        let old = self.pop();
        debug_assert_eq!(old.wide(), t.wide());
        self.push(t);
    }

    /// Reserves the next free slot (two for a long or double) and returns it.
    pub fn new_local(&mut self, t: VT) -> u16 {
        let slot = self.locals.len() as u16;
        self.locals.push(t);
        if t.wide() {
            self.locals.push(VT::Top2);
        }
        self.max_locals = self.max_locals.max(self.locals.len() as u16);
        slot
    }

    pub fn locals_mark(&self) -> usize {
        self.locals.len()
    }

    /// Leaves a scope: the slots above the mark are free again.
    pub fn locals_release(&mut self, mark: usize) {
        self.locals.truncate(mark);
    }

    // ---- labels and frames ----

    pub fn new_label(&mut self) -> Label {
        self.labels.push(LabelInfo { offset: None, locals_len: self.locals.len(), frame: None });
        Label(self.labels.len() as u32 - 1)
    }

    fn record_target(&mut self, l: Label) {
        let info = &mut self.labels[l.0 as usize];
        if info.frame.is_none() {
            let mut locals = self.locals.clone();
            locals.truncate(info.locals_len);
            info.frame = Some((locals, self.stack.clone()));
        }
    }

    /// Binds the label here. The state becomes the one recorded at the first jump to it; code
    /// that falls into it has to agree with that state.
    pub fn bind(&mut self, l: Label) {
        let offset = self.pc();
        let info = &mut self.labels[l.0 as usize];
        info.offset = Some(offset);
        let (locals, stack) = match info.frame.take() {
            Some((mut locals, stack)) => {
                // A slot released since the jump holds nothing the code after the label reads.
                if self.reachable && locals.len() > self.locals.len() {
                    locals.truncate(self.locals.len());
                }
                (locals, stack)
            }
            None => (self.locals.clone(), self.stack.clone()),
        };
        debug_assert!(!self.reachable || stack.len() == self.stack.len(), "stack depth differs at a label");
        self.locals = locals;
        self.depth = stack.iter().map(|t| if t.wide() { 2 } else { 1 }).sum();
        self.max_stack = self.max_stack.max(self.depth);
        self.stack = stack;
        self.labels[l.0 as usize].frame = Some((self.locals.clone(), self.stack.clone()));
        self.reachable = true;
        self.add_frame(offset);
    }

    fn add_frame(&mut self, offset: u32) {
        let frame = Frame { offset, locals: self.locals.clone(), stack: self.stack.clone() };
        match self.frames.last_mut() {
            Some(last) if last.offset == offset => *last = frame,
            _ => self.frames.push(frame),
        }
    }

    /// Code after an unconditional transfer that no label leads to is unreachable, and the
    /// verifier still wants a frame in front of it.
    fn before_insn(&mut self) {
        if !self.reachable {
            self.reachable = true;
            let offset = self.pc();
            self.add_frame(offset);
        }
    }

    pub fn add_handler(&mut self, h: Handler) {
        self.handlers.push(h);
    }

    /// Binds the entry of an exception handler: the locals as they are, the exception on the stack.
    pub fn bind_handler(&mut self, l: Label, exception: VT) {
        self.stack.clear();
        self.depth = 0;
        self.push(exception);
        self.labels[l.0 as usize].frame = None;
        self.reachable = false;
        self.bind(l);
    }

    // ---- instructions ----

    pub fn op(&mut self, opcode: u8) {
        self.before_insn();
        self.bytes.push(opcode);
    }

    pub fn op_u8(&mut self, opcode: u8, operand: u8) {
        self.before_insn();
        self.bytes.push(opcode);
        self.bytes.push(operand);
    }

    pub fn op_u16(&mut self, opcode: u8, operand: u16) {
        self.before_insn();
        self.bytes.push(opcode);
        self.bytes.extend_from_slice(&operand.to_be_bytes());
    }

    /// A load or store with its slot, in the short, plain or wide form.
    pub fn var_insn(&mut self, base: u8, short_base: u8, slot: u16) {
        self.before_insn();
        if slot <= 3 {
            self.bytes.push(short_base + slot as u8);
        } else if slot <= 255 {
            self.bytes.push(base);
            self.bytes.push(slot as u8);
        } else {
            self.bytes.push(0xc4);
            self.bytes.push(base);
            self.bytes.extend_from_slice(&slot.to_be_bytes());
        }
    }

    /// A branch instruction; the operands it pops are already gone from the model.
    pub fn jump(&mut self, opcode: u8, target: Label) {
        self.before_insn();
        if self.wide {
            if opcode != op::GOTO {
                let inverse = match opcode {
                    op::IFNULL => op::IFNONNULL,
                    op::IFNONNULL => op::IFNULL,
                    o if (o - op::IFEQ) % 2 == 0 => o + 1,
                    o => o - 1,
                };
                self.bytes.push(inverse);
                self.bytes.extend_from_slice(&8u16.to_be_bytes());
            }
            let at = self.pc();
            self.bytes.push(0xc8);
            self.fixups.push((at, (at + 1) | 0x8000_0000, target));
            self.bytes.extend_from_slice(&[0, 0, 0, 0]);
            self.record_target(target);
            if opcode == op::GOTO {
                self.reachable = false;
            } else {
                // The inverse lands behind the `goto_w`.
                let offset = self.pc();
                self.add_frame(offset);
            }
            return;
        }
        let at = self.pc();
        self.bytes.push(opcode);
        self.fixups.push((at, at + 1, target));
        self.bytes.extend_from_slice(&[0, 0]);
        self.record_target(target);
        if opcode == op::GOTO {
            self.reachable = false;
        }
    }

    /// Marks the end of a path: after a return or `athrow`.
    pub fn end_path(&mut self) {
        self.reachable = false;
    }

    fn finish(mut self, cp: &mut ConstPool) -> (Vec<u8>, bool) {
        if self.bytes.is_empty() || !self.reachable && self.frames.last().map_or(false, |f| f.offset == self.pc()) {
            // A label bound at the very end needs an instruction to stand on.
            self.frames.retain(|f| f.offset < self.bytes.len() as u32 || self.bytes.is_empty());
        }
        for &(insn, pos, label) in &self.fixups {
            let target = self.labels[label.0 as usize].offset.expect("unbound label") as i64;
            let delta = target - insn as i64;
            if pos & 0x8000_0000 != 0 {
                let p = (pos & 0x7fff_ffff) as usize;
                self.bytes[p..p + 4].copy_from_slice(&(delta as i32).to_be_bytes());
            } else {
                if delta < i16::MIN as i64 || delta > i16::MAX as i64 {
                    self.overflow = true;
                    continue;
                }
                let p = pos as usize;
                self.bytes[p..p + 2].copy_from_slice(&(delta as i16).to_be_bytes());
            }
        }
        let code_len = self.bytes.len() as u32;
        let mut out = Vec::with_capacity(self.bytes.len() + 64);
        out.extend_from_slice(&self.max_stack.to_be_bytes());
        out.extend_from_slice(&self.max_locals.to_be_bytes());
        out.extend_from_slice(&code_len.to_be_bytes());
        out.extend_from_slice(&self.bytes);
        out.extend_from_slice(&(self.handlers.len() as u16).to_be_bytes());
        for h in &self.handlers {
            for l in [h.start, h.end, h.handler] {
                let o = self.labels[l.0 as usize].offset.expect("unbound label") as u16;
                out.extend_from_slice(&o.to_be_bytes());
            }
            out.extend_from_slice(&h.catch_type.to_be_bytes());
        }
        let frames: Vec<&Frame> = self.frames.iter().filter(|f| f.offset < code_len).collect();
        let attr_count = (!frames.is_empty()) as u16 + (!self.lines.is_empty()) as u16;
        out.extend_from_slice(&attr_count.to_be_bytes());
        if !frames.is_empty() {
            let name = cp.utf8("StackMapTable");
            let mut body = Vec::with_capacity(frames.len() * 8);
            body.extend_from_slice(&(frames.len() as u16).to_be_bytes());
            let mut previous: i64 = -1;
            let mut before: Vec<VT> = self.initial_locals.iter().copied().filter(|t| *t != VT::Top2).collect();
            for f in frames {
                let delta = (f.offset as i64 - previous - 1) as u16;
                previous = f.offset as i64;
                let locals: Vec<VT> = f.locals.iter().copied().filter(|t| *t != VT::Top2).collect();
                let common = before.iter().zip(&locals).take_while(|(a, b)| a == b).count();
                let appended = locals.len() - common;
                let chopped = before.len() - common;
                if f.stack.is_empty() && appended == 0 && chopped == 0 {
                    // same_frame
                    if delta < 64 {
                        body.push(delta as u8);
                    } else {
                        body.push(251);
                        body.extend_from_slice(&delta.to_be_bytes());
                    }
                } else if f.stack.len() == 1 && appended == 0 && chopped == 0 {
                    // same_locals_1_stack_item
                    if delta < 64 {
                        body.push(64 + delta as u8);
                    } else {
                        body.push(247);
                        body.extend_from_slice(&delta.to_be_bytes());
                    }
                    write_vt(&mut body, f.stack[0]);
                } else if f.stack.is_empty() && chopped == 0 && appended <= 3 {
                    // append_frame
                    body.push(251 + appended as u8);
                    body.extend_from_slice(&delta.to_be_bytes());
                    for &t in &locals[common..] {
                        write_vt(&mut body, t);
                    }
                } else if f.stack.is_empty() && appended == 0 && chopped <= 3 {
                    // chop_frame
                    body.push(251 - chopped as u8);
                    body.extend_from_slice(&delta.to_be_bytes());
                } else {
                    body.push(255);
                    body.extend_from_slice(&delta.to_be_bytes());
                    body.extend_from_slice(&(locals.len() as u16).to_be_bytes());
                    for &t in &locals {
                        write_vt(&mut body, t);
                    }
                    body.extend_from_slice(&(f.stack.len() as u16).to_be_bytes());
                    for &t in &f.stack {
                        write_vt(&mut body, t);
                    }
                }
                before = locals;
            }
            out.extend_from_slice(&name.to_be_bytes());
            out.extend_from_slice(&(body.len() as u32).to_be_bytes());
            out.extend_from_slice(&body);
        }
        if !self.lines.is_empty() {
            let name = cp.utf8("LineNumberTable");
            out.extend_from_slice(&name.to_be_bytes());
            out.extend_from_slice(&((2 + self.lines.len() * 4) as u32).to_be_bytes());
            out.extend_from_slice(&(self.lines.len() as u16).to_be_bytes());
            for &(pc, line) in &self.lines {
                out.extend_from_slice(&pc.to_be_bytes());
                out.extend_from_slice(&line.to_be_bytes());
            }
        }
        let overflow = self.overflow || code_len > 65535;
        (out, overflow)
    }
}

fn write_vt(out: &mut Vec<u8>, t: VT) {
    match t {
        VT::Top | VT::Top2 => out.push(0),
        VT::Int => out.push(1),
        VT::Float => out.push(2),
        VT::Double => out.push(3),
        VT::Long => out.push(4),
        VT::Null => out.push(5),
        VT::UninitThis => out.push(6),
        VT::Object(c) => {
            out.push(7);
            out.extend_from_slice(&c.to_be_bytes());
        }
        VT::Uninit(offset) => {
            out.push(8);
            out.extend_from_slice(&offset.to_be_bytes());
        }
    }
}

struct Member {
    access: u16,
    name: u16,
    desc: u16,
    attrs: Vec<(u16, Vec<u8>)>,
}

pub struct ClassWriter {
    pub cp: ConstPool,
    access: u16,
    this_class: u16,
    super_class: u16,
    interfaces: Vec<u16>,
    fields: Vec<Member>,
    methods: Vec<Member>,
    attrs: Vec<(u16, Vec<u8>)>,
    bootstrap: Vec<Vec<u8>>,
    /// Whether a method had a jump that its 16-bit offset could not express.
    pub overflow: bool,
}

impl ClassWriter {
    pub fn new(access: u16, name: &str, super_name: &str, interfaces: &[String]) -> ClassWriter {
        let mut cp = ConstPool::new();
        let this_class = cp.class(name);
        let super_class = cp.class(super_name);
        let interfaces = interfaces.iter().map(|i| cp.class(i)).collect();
        ClassWriter {
            cp,
            access,
            this_class,
            super_class,
            interfaces,
            fields: Vec::new(),
            methods: Vec::new(),
            attrs: Vec::new(),
            bootstrap: Vec::new(),
            overflow: false,
        }
    }

    pub fn source_file(&mut self, file: &str) {
        let name = self.cp.utf8("SourceFile");
        let value = self.cp.utf8(file);
        self.attrs.push((name, value.to_be_bytes().to_vec()));
    }

    pub fn field(&mut self, access: u16, name: &str, desc: &str) {
        let name = self.cp.utf8(name);
        let desc = self.cp.utf8(desc);
        self.fields.push(Member { access, name, desc, attrs: Vec::new() });
    }

    pub fn has_method(&self, name: &str, desc: &str) -> bool {
        let (Some(&n), Some(&d)) = (self.cp.utf8.get(name), self.cp.utf8.get(desc)) else { return false };
        self.methods.iter().any(|m| m.name == n && m.desc == d)
    }

    pub fn has_method_named(&self, name: &str) -> bool {
        let Some(&n) = self.cp.utf8.get(name) else { return false };
        self.methods.iter().any(|m| m.name == n)
    }

    /// The access flags, names and descriptors of the methods written so far.
    pub fn method_table(&self) -> Vec<(u16, String, String)> {
        let mut text: FxMap<u16, &str> = FxMap::default();
        for (s, &i) in &self.cp.utf8 {
            text.insert(i, s);
        }
        self.methods.iter().map(|m| (m.access, text[&m.name].to_string(), text[&m.desc].to_string())).collect()
    }

    pub fn method(&mut self, access: u16, name: &str, desc: &str, code: Option<Code>) {
        let name = self.cp.utf8(name);
        let desc = self.cp.utf8(desc);
        let mut attrs = Vec::new();
        if let Some(code) = code {
            let attr = self.cp.utf8("Code");
            let (bytes, overflow) = code.finish(&mut self.cp);
            self.overflow |= overflow;
            attrs.push((attr, bytes));
        }
        self.methods.push(Member { access, name, desc, attrs });
    }

    /// A bootstrap method with its static arguments (constant pool indices); returns its index.
    pub fn bootstrap_method(&mut self, handle: u16, args: &[u16]) -> u16 {
        let mut entry = Vec::with_capacity(4 + args.len() * 2);
        entry.extend_from_slice(&handle.to_be_bytes());
        entry.extend_from_slice(&(args.len() as u16).to_be_bytes());
        for a in args {
            entry.extend_from_slice(&a.to_be_bytes());
        }
        if let Some(i) = self.bootstrap.iter().position(|e| *e == entry) {
            return i as u16;
        }
        self.bootstrap.push(entry);
        self.bootstrap.len() as u16 - 1
    }

    /// The class file at the major version of `output_version`, a Java release.
    pub fn finish(mut self, output_version: u32) -> Vec<u8> {
        if !self.bootstrap.is_empty() {
            let name = self.cp.utf8("BootstrapMethods");
            let mut body = Vec::new();
            body.extend_from_slice(&(self.bootstrap.len() as u16).to_be_bytes());
            for e in &self.bootstrap {
                body.extend_from_slice(e);
            }
            self.attrs.push((name, body));
        }
        let mut out = Vec::with_capacity(self.cp.bytes.len() + 512);
        out.extend_from_slice(&0xCAFEBABEu32.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes());
        out.extend_from_slice(&major_version(output_version).to_be_bytes());
        out.extend_from_slice(&self.cp.count.to_be_bytes());
        out.extend_from_slice(&self.cp.bytes);
        out.extend_from_slice(&self.access.to_be_bytes());
        out.extend_from_slice(&self.this_class.to_be_bytes());
        out.extend_from_slice(&self.super_class.to_be_bytes());
        out.extend_from_slice(&(self.interfaces.len() as u16).to_be_bytes());
        for i in &self.interfaces {
            out.extend_from_slice(&i.to_be_bytes());
        }
        for members in [&self.fields, &self.methods] {
            out.extend_from_slice(&(members.len() as u16).to_be_bytes());
            for m in members {
                out.extend_from_slice(&m.access.to_be_bytes());
                out.extend_from_slice(&m.name.to_be_bytes());
                out.extend_from_slice(&m.desc.to_be_bytes());
                write_attrs(&mut out, &m.attrs);
            }
        }
        write_attrs(&mut out, &self.attrs);
        out
    }
}

fn write_attrs(out: &mut Vec<u8>, attrs: &[(u16, Vec<u8>)]) {
    out.extend_from_slice(&(attrs.len() as u16).to_be_bytes());
    for (name, body) in attrs {
        out.extend_from_slice(&name.to_be_bytes());
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
    }
}

pub mod op {
    pub const NOP: u8 = 0x00;
    pub const ACONST_NULL: u8 = 0x01;
    pub const ICONST_0: u8 = 0x03;
    pub const LCONST_0: u8 = 0x09;
    pub const FCONST_0: u8 = 0x0b;
    pub const DCONST_0: u8 = 0x0e;
    pub const BIPUSH: u8 = 0x10;
    pub const SIPUSH: u8 = 0x11;
    pub const LDC: u8 = 0x12;
    pub const LDC_W: u8 = 0x13;
    pub const LDC2_W: u8 = 0x14;
    pub const ILOAD: u8 = 0x15;
    pub const LLOAD: u8 = 0x16;
    pub const FLOAD: u8 = 0x17;
    pub const DLOAD: u8 = 0x18;
    pub const ALOAD: u8 = 0x19;
    pub const ILOAD_0: u8 = 0x1a;
    pub const LLOAD_0: u8 = 0x1e;
    pub const FLOAD_0: u8 = 0x22;
    pub const DLOAD_0: u8 = 0x26;
    pub const ALOAD_0: u8 = 0x2a;
    pub const ISTORE: u8 = 0x36;
    pub const LSTORE: u8 = 0x37;
    pub const FSTORE: u8 = 0x38;
    pub const DSTORE: u8 = 0x39;
    pub const ASTORE: u8 = 0x3a;
    pub const ISTORE_0: u8 = 0x3b;
    pub const LSTORE_0: u8 = 0x3f;
    pub const FSTORE_0: u8 = 0x43;
    pub const DSTORE_0: u8 = 0x47;
    pub const ASTORE_0: u8 = 0x4b;
    pub const IASTORE: u8 = 0x4f;
    pub const LASTORE: u8 = 0x50;
    pub const FASTORE: u8 = 0x51;
    pub const DASTORE: u8 = 0x52;
    pub const AASTORE: u8 = 0x53;
    pub const BASTORE: u8 = 0x54;
    pub const CASTORE: u8 = 0x55;
    pub const SASTORE: u8 = 0x56;
    pub const POP: u8 = 0x57;
    pub const POP2: u8 = 0x58;
    pub const DUP: u8 = 0x59;
    pub const DUP2: u8 = 0x5c;
    pub const IADD: u8 = 0x60;
    pub const LADD: u8 = 0x61;
    pub const FADD: u8 = 0x62;
    pub const DADD: u8 = 0x63;
    pub const ISUB: u8 = 0x64;
    pub const LSUB: u8 = 0x65;
    pub const FSUB: u8 = 0x66;
    pub const DSUB: u8 = 0x67;
    pub const IMUL: u8 = 0x68;
    pub const LMUL: u8 = 0x69;
    pub const FMUL: u8 = 0x6a;
    pub const DMUL: u8 = 0x6b;
    pub const IDIV: u8 = 0x6c;
    pub const LDIV: u8 = 0x6d;
    pub const FDIV: u8 = 0x6e;
    pub const DDIV: u8 = 0x6f;
    pub const IREM: u8 = 0x70;
    pub const LREM: u8 = 0x71;
    pub const FREM: u8 = 0x72;
    pub const DREM: u8 = 0x73;
    pub const INEG: u8 = 0x74;
    pub const LNEG: u8 = 0x75;
    pub const FNEG: u8 = 0x76;
    pub const DNEG: u8 = 0x77;
    pub const ISHL: u8 = 0x78;
    pub const LSHL: u8 = 0x79;
    pub const ISHR: u8 = 0x7a;
    pub const LSHR: u8 = 0x7b;
    pub const IUSHR: u8 = 0x7c;
    pub const LUSHR: u8 = 0x7d;
    pub const IAND: u8 = 0x7e;
    pub const LAND: u8 = 0x7f;
    pub const IOR: u8 = 0x80;
    pub const LOR: u8 = 0x81;
    pub const IXOR: u8 = 0x82;
    pub const LXOR: u8 = 0x83;
    pub const I2L: u8 = 0x85;
    pub const I2F: u8 = 0x86;
    pub const I2D: u8 = 0x87;
    pub const L2I: u8 = 0x88;
    pub const L2F: u8 = 0x89;
    pub const L2D: u8 = 0x8a;
    pub const F2I: u8 = 0x8b;
    pub const F2L: u8 = 0x8c;
    pub const F2D: u8 = 0x8d;
    pub const D2I: u8 = 0x8e;
    pub const D2L: u8 = 0x8f;
    pub const D2F: u8 = 0x90;
    pub const I2B: u8 = 0x91;
    pub const I2C: u8 = 0x92;
    pub const I2S: u8 = 0x93;
    pub const LCMP: u8 = 0x94;
    pub const FCMPL: u8 = 0x95;
    pub const FCMPG: u8 = 0x96;
    pub const DCMPL: u8 = 0x97;
    pub const DCMPG: u8 = 0x98;
    pub const IFEQ: u8 = 0x99;
    pub const IFNE: u8 = 0x9a;
    pub const IFLT: u8 = 0x9b;
    pub const IFGE: u8 = 0x9c;
    pub const IFGT: u8 = 0x9d;
    pub const IFLE: u8 = 0x9e;
    pub const IF_ICMPEQ: u8 = 0x9f;
    pub const IF_ICMPNE: u8 = 0xa0;
    pub const IF_ICMPLT: u8 = 0xa1;
    pub const IF_ICMPGE: u8 = 0xa2;
    pub const IF_ICMPGT: u8 = 0xa3;
    pub const IF_ICMPLE: u8 = 0xa4;
    pub const IF_ACMPEQ: u8 = 0xa5;
    pub const IF_ACMPNE: u8 = 0xa6;
    pub const GOTO: u8 = 0xa7;
    pub const IRETURN: u8 = 0xac;
    pub const LRETURN: u8 = 0xad;
    pub const FRETURN: u8 = 0xae;
    pub const DRETURN: u8 = 0xaf;
    pub const ARETURN: u8 = 0xb0;
    pub const RETURN: u8 = 0xb1;
    pub const GETSTATIC: u8 = 0xb2;
    pub const PUTSTATIC: u8 = 0xb3;
    pub const GETFIELD: u8 = 0xb4;
    pub const PUTFIELD: u8 = 0xb5;
    pub const INVOKEVIRTUAL: u8 = 0xb6;
    pub const INVOKESPECIAL: u8 = 0xb7;
    pub const INVOKESTATIC: u8 = 0xb8;
    pub const INVOKEINTERFACE: u8 = 0xb9;
    pub const INVOKEDYNAMIC: u8 = 0xba;
    pub const NEW: u8 = 0xbb;
    pub const IALOAD: u8 = 0x2e;
    pub const LALOAD: u8 = 0x2f;
    pub const FALOAD: u8 = 0x30;
    pub const DALOAD: u8 = 0x31;
    pub const AALOAD: u8 = 0x32;
    pub const BALOAD: u8 = 0x33;
    pub const CALOAD: u8 = 0x34;
    pub const SALOAD: u8 = 0x35;
    pub const ARRAYLENGTH: u8 = 0xbe;
    pub const NEWARRAY: u8 = 0xbc;
    pub const ANEWARRAY: u8 = 0xbd;
    pub const ATHROW: u8 = 0xbf;
    pub const CHECKCAST: u8 = 0xc0;
    pub const INSTANCEOF: u8 = 0xc1;
    pub const IFNULL: u8 = 0xc6;
    pub const IFNONNULL: u8 = 0xc7;
}

/// A jar is a zip archive; the entries are stored without compression.
pub fn write_jar(path: &str, main_class: Option<&str>, classes: &[(String, Vec<u8>)]) -> std::io::Result<()> {
    let mut entries: Vec<(String, std::borrow::Cow<[u8]>)> = Vec::with_capacity(classes.len() + 1);
    let mut manifest = String::from("Manifest-Version: 1.0\r\nCreated-By: teq\r\n");
    if let Some(m) = main_class {
        manifest.push_str(&format!("Main-Class: {}\r\n", m.replace('/', ".")));
    }
    manifest.push_str("\r\n");
    entries.push(("META-INF/MANIFEST.MF".to_string(), manifest.into_bytes().into()));
    for (name, bytes) in classes {
        entries.push((format!("{}.class", name), bytes.as_slice().into()));
    }
    let total: usize = entries.iter().map(|(n, b)| b.len() + 2 * n.len() + 76).sum();
    let mut out: Vec<u8> = Vec::with_capacity(total + 22);
    let mut central: Vec<u8> = Vec::new();
    let table = crc_table();
    // The checksums are most of the work of a stored archive; the entries share no state.
    let workers = crate::workers().min(entries.len().max(1));
    let per_worker = entries.len().div_ceil(workers).max(1);
    let crcs: Vec<u32> = if entries.len() < 256 || workers == 1 {
        entries.iter().map(|(_, data)| crc32(&table, data)).collect()
    } else {
        std::thread::scope(|scope| {
            let handles: Vec<_> = entries
                .chunks(per_worker)
                .map(|chunk| {
                    let table = &table;
                    crate::alloc::spawn_in(scope, move || chunk.iter().map(|(_, data)| crc32(table, data)).collect::<Vec<u32>>())
                })
                .collect();
            handles.into_iter().flat_map(|h| h.join().expect("checksum thread panicked")).collect()
        })
    };
    for ((name, data), &crc) in entries.iter().zip(&crcs) {
        let offset = out.len() as u32;
        let size = data.len() as u32;
        let header = |out: &mut Vec<u8>| {
            out.extend_from_slice(&[10, 0, 0, 8, 0, 0]);
            out.extend_from_slice(&[0, 0, 0x21, 0]);
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        };
        out.extend_from_slice(&0x04034b50u32.to_le_bytes());
        header(&mut out);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        central.extend_from_slice(&0x02014b50u32.to_le_bytes());
        central.extend_from_slice(&[10, 0]);
        header(&mut central);
        central.extend_from_slice(&[0; 12]);
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let central_offset = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x06054b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&central_offset.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, out)
}

fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for i in 0..256u32 {
        let mut c = i;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb88320 ^ (c >> 1) } else { c >> 1 };
        }
        table[i as usize] = c;
    }
    table
}

fn crc32(table: &[u32; 256], data: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c = table[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}
