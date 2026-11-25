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

pub const ACC_SUPER: u16 = 0x0020;
pub const ACC_BRIDGE: u16 = 0x0040;
/// The bit of `ACC_BRIDGE` on a field.
pub const ACC_VOLATILE: u16 = 0x0040;
pub const ACC_INTERFACE: u16 = 0x0200;
pub const ACC_ABSTRACT: u16 = 0x0400;
pub const ACC_SYNTHETIC: u16 = 0x1000;
pub const ACC_ENUM: u16 = 0x4000;

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

/// The largest `constant_pool_count` a class file holds: its entries take the indexes from 1 to
/// 65,534, a long or a double two of them (JVMS 4.1, 4.4.5).
pub const MAX_POOL_COUNT: u32 = 65_535;

/// The bound of a class file a constant pool passed (`ConstPool::unwritable`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PoolLimit {
    /// More entries than `MAX_POOL_COUNT` allows.
    Entries,
    /// A text of more than `MAX_TEXT_BYTES` bytes.
    Text,
}

#[derive(Default)]
pub struct ConstPool {
    bytes: Vec<u8>,
    /// The next entry's index: the class's `constant_pool_count` while it is at most
    /// `MAX_POOL_COUNT`, the pool too large for a class file past it, whose indexes are then no
    /// one's (`too_large`).
    count: u32,
    /// A text longer than a constant holds was added, its length then no one's.
    long_text: bool,
    utf8: FxMap<Box<str>, u16>,
    consts: FxMap<Const, u16>,
    /// A lowering runs (`Gen::lowering`): nothing may enter the pool, whose order is the output's.
    pub frozen: bool,
    /// In checks builds, a class's plan is being made outside its declarations (`Gen::plan_phase`): what enters the pool
    /// then would enter it before the code of the methods written so far once the class is written from its plan.
    pub plan_frozen: bool,
}

impl ConstPool {
    fn new() -> ConstPool {
        ConstPool { bytes: Vec::with_capacity(1024), count: 1, long_text: false, utf8: FxMap::default(), consts: FxMap::default(), frozen: false, plan_frozen: false }
    }

    pub fn utf8(&mut self, s: &str) -> u16 {
        if let Some(&i) = self.utf8.get(s) {
            return i;
        }
        assert!(!self.frozen, "the constant pool is written while a lowering runs");
        debug_assert!(!self.plan_frozen, "the constant pool is written while a class's plan is made");
        let i = self.take(1);
        self.bytes.push(1);
        let encoded = modified_utf8(s);
        self.long_text |= encoded.len() > MAX_TEXT_BYTES;
        self.bytes.extend_from_slice(&(encoded.len() as u16).to_be_bytes());
        self.bytes.extend_from_slice(&encoded);
        self.utf8.insert(s.into(), i);
        i
    }

    fn add(&mut self, c: Const) -> u16 {
        if let Some(&i) = self.consts.get(&c) {
            return i;
        }
        assert!(!self.frozen, "the constant pool is written while a lowering runs");
        debug_assert!(!self.plan_frozen, "the constant pool is written while a class's plan is made");
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
        let i = self.take(if wide { 2 } else { 1 });
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
        let i = self.take(1);
        self.bytes.push(tag);
        self.bytes.extend_from_slice(&class.to_be_bytes());
        self.bytes.extend_from_slice(&nt.to_be_bytes());
        self.consts.insert(c, i);
        i
    }

    /// The index of an entry of `slots` slots added now. Past the class file's bound it is no
    /// entry's, and the class is not written (`ClassWriter::finish`).
    fn take(&mut self, slots: u32) -> u16 {
        let i = self.count;
        self.count += slots;
        i as u16
    }

    /// Whether the pool holds more entries than a class file can.
    pub fn too_large(&self) -> bool {
        self.count > MAX_POOL_COUNT
    }

    /// What of the pool a class file cannot hold, the first that ASM's `ClassWriter.toByteArray`
    /// meets: a text past a constant's bytes as it adds it (`ByteVector.putUTF8`), then the count.
    pub fn unwritable(&self) -> Option<PoolLimit> {
        if self.long_text {
            Some(PoolLimit::Text)
        } else if self.too_large() {
            Some(PoolLimit::Entries)
        } else {
            None
        }
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
    /// `a` as a class file's annotation (JVMS 4.7.16), its names and constants in this pool.
    pub fn annotation(&mut self, a: &crate::classfile::Annotation, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.utf8(&a.descriptor).to_be_bytes());
        out.extend_from_slice(&(a.elements.len() as u16).to_be_bytes());
        for (name, value) in &a.elements {
            out.extend_from_slice(&self.utf8(name).to_be_bytes());
            self.element_value(value, out);
        }
    }

    fn element_value(&mut self, v: &crate::classfile::ElementValue, out: &mut Vec<u8>) {
        use crate::classfile::{ConstValue, ElementValue};
        match v {
            ElementValue::Const(tag, c) => {
                out.push(*tag);
                let i = match c {
                    ConstValue::Int(v) => self.int(*v),
                    ConstValue::Long(v) => self.long(*v),
                    ConstValue::Float(v) => self.float(*v),
                    ConstValue::Double(v) => self.double(*v),
                    ConstValue::Str(s) => self.utf8(s),
                };
                out.extend_from_slice(&i.to_be_bytes());
            }
            ElementValue::Enum(ty, name) => {
                out.push(b'e');
                out.extend_from_slice(&self.utf8(ty).to_be_bytes());
                out.extend_from_slice(&self.utf8(name).to_be_bytes());
            }
            ElementValue::Class(desc) => {
                out.push(b'c');
                out.extend_from_slice(&self.utf8(desc).to_be_bytes());
            }
            ElementValue::Annotation(a) => {
                out.push(b'@');
                self.annotation(a, out);
            }
            ElementValue::Array(values) => {
                out.push(b'[');
                out.extend_from_slice(&(values.len() as u16).to_be_bytes());
                for v in values {
                    self.element_value(v, out);
                }
            }
        }
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

/// What a class file's text constant holds, in bytes (JVMS 4.4.7).
pub const MAX_TEXT_BYTES: usize = 65_535;

/// dotty's `BCodeUtils.checkConstantStringLength` (BCodeUtils.scala 41-60): whether `name` and
/// `other` together stay within the bytes of one text constant, counted as dotty counts them: a
/// NUL two bytes, a character of the BMP its modified UTF-8 length, each half of a surrogate pair
/// two.
pub fn constant_text_fits(name: &str, other: &str) -> bool {
    if name.len() + other.len() <= MAX_TEXT_BYTES / 3 {
        return true;
    }
    let mut bytes = 0;
    for unit in crate::text::utf16_units(name).chain(crate::text::utf16_units(other)) {
        bytes += match unit {
            0 => 2,
            0x0001..=0x007f => 1,
            0x0080..=0x07ff | 0xd800..=0xdfff => 2,
            _ => 3,
        };
        if bytes > MAX_TEXT_BYTES {
            return false;
        }
    }
    true
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

/// What `finish` lays out: a jump as it is written first, 16-bit, or a switch, whose padding follows its offset.
enum Item {
    Jump { at: u32, target: Label },
    /// The opcode at `at`, `pad` zero bytes after it as first written, its 32-bit offsets at `slots` (each from `at`
    /// to its label), its operands ending at `end`.
    Switch { at: u32, pad: u32, end: u32, slots: Vec<(u32, Label)> },
}

impl Item {
    fn at(&self) -> u32 {
        match *self {
            Item::Jump { at, .. } | Item::Switch { at, .. } => at,
        }
    }
}

/// The zero bytes after a switch's opcode at `at`: its operands start at a multiple of four (JVMS 6.5).
fn switch_pad(at: u32) -> u32 {
    (4 - (at + 1) % 4) % 4
}

/// The state a conditional jump falls through with (`Code::relax`): the frame its inverse lands
/// on where the jump is widened.
struct Falls {
    jump: usize,
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

/// Slots taken before the value they hold is computed (`Code::reserve_local`), as many as that value takes.
#[derive(Clone, Copy)]
pub struct Reserved {
    pub slot: u16,
    wide: bool,
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
    items: Vec<Item>,
    falls: Vec<Falls>,
    frames: Vec<Frame>,
    handlers: Vec<Handler>,
    lines: Vec<(u16, u16)>,
    /// False after an unconditional transfer until a label is bound.
    pub reachable: bool,
    /// The method is written again because a jump of its unit did not reach in 16 bits: each
    /// conditional jump keeps the state it falls through with, so that `finish` can widen the
    /// jumps that do not reach (`Code::widen`).
    pub relax: bool,
    /// Set by `finish` when a 16-bit offset did not reach its target outside `relax`, or the code
    /// is longer than the JVM allows.
    pub overflow: bool,
    /// Set when a pop met an empty model: the method's code is wrong and is reported, not
    /// written.
    pub underflow: bool,
    /// Set when a value was stored in slots `reserve_local` took for another width: the method's code is wrong and
    /// is reported, not written.
    pub misreserved: bool,
    /// A lowering runs (`Gen::lowering`): nothing may be written, since what a lowering decides
    /// must not depend on what is written, nor the output on the order of the lowerings.
    pub frozen: bool,
    /// The nodes being written around the next instruction (`Gen::write`): every instruction is a node's, which a debug
    /// build checks as it is written.
    pub in_node: u32,
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
            items: Vec::new(),
            falls: Vec::new(),
            frames: Vec::new(),
            handlers: Vec::new(),
            lines: Vec::new(),
            reachable: true,
            relax: false,
            overflow: false,
            underflow: false,
            misreserved: false,
            frozen: false,
            in_node: 0,
        }
    }

    #[inline]
    fn writing(&self) {
        assert!(!self.frozen, "the code is written while a lowering runs");
    }

    pub fn pc(&self) -> u32 {
        self.bytes.len() as u32
    }

    /// The source line the code being written is at, none before the first.
    pub fn current_line(&self) -> Option<u16> {
        self.lines.last().map(|&(_, line)| line)
    }

    pub fn line(&mut self, line: u32) {
        self.writing();
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
        self.writing();
        self.depth += if t.wide() { 2 } else { 1 };
        self.max_stack = self.max_stack.max(self.depth);
        self.stack.push(t);
    }

    /// The source line the code last recorded, for a report.
    pub fn last_line(&self) -> Option<u16> {
        self.lines.last().map(|&(_, l)| l)
    }

    pub fn pop(&mut self) -> VT {
        self.writing();
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
        self.writing();
        let old = self.pop();
        debug_assert_eq!(old.wide(), t.wide());
        self.push(t);
    }

    /// Reserves the next free slot (two for a long or double) and returns it.
    pub fn new_local(&mut self, t: VT) -> u16 {
        self.writing();
        let slot = self.locals.len() as u16;
        self.locals.push(t);
        if t.wide() {
            self.locals.push(VT::Top2);
        }
        self.max_locals = self.max_locals.max(self.locals.len() as u16);
        slot
    }

    /// Takes the next free slot for a value computed later, two for a long or a double (dotty makes a val's local by
    /// its kind's size before its right side: `makeLocal`, BCodeSkelBuilder.scala 582-591 at 3.8.4). They are unset
    /// until `type_local` types them.
    pub fn reserve_local(&mut self, wide: bool) -> Reserved {
        self.writing();
        let slot = self.locals.len() as u16;
        self.locals.push(VT::Top);
        if wide {
            self.locals.push(VT::Top);
        }
        self.max_locals = self.max_locals.max(self.locals.len() as u16);
        Reserved { slot, wide }
    }

    /// Gives the slots `reserve_local` took the type `t` from here on, as the value's first store does: a long's or a
    /// double's second slot its second half.
    pub fn type_local(&mut self, reserved: Reserved, t: VT) {
        self.writing();
        if reserved.wide != t.wide() {
            self.misreserved = true;
            return;
        }
        self.locals[reserved.slot as usize] = t;
        if reserved.wide {
            self.locals[reserved.slot as usize + 1] = VT::Top2;
        }
    }

    /// Holds a slot taken before the label just bound, whose frame (cut to the locals at the
    /// label's making) does not have it: the locals made from here take the slots after it, and a
    /// later frame records it as unset.
    pub fn hold_local(&mut self, slot: u16) {
        self.writing();
        while self.locals.len() <= slot as usize {
            self.locals.push(VT::Top);
        }
        self.max_locals = self.max_locals.max(self.locals.len() as u16);
    }

    pub fn locals_mark(&self) -> usize {
        self.locals.len()
    }

    /// Leaves a scope: the slots above the mark are free again.
    pub fn locals_release(&mut self, mark: usize) {
        self.writing();
        self.locals.truncate(mark);
    }

    // ---- labels and frames ----

    pub fn new_label(&mut self) -> Label {
        self.writing();
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
        self.writing();
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
    /// Records the frame an instruction written now stands on where no path reaches it, while the
    /// model still holds the operands the instruction takes: a branch after a `return` or a jump
    /// (`Gen::jump_if`) pops its operands from the model before it is written.
    pub fn reach(&mut self) {
        self.before_insn();
    }

    fn before_insn(&mut self) {
        self.writing();
        debug_assert!(self.in_node > 0, "an instruction written outside a node");
        if !self.reachable {
            self.reachable = true;
            let offset = self.pc();
            self.add_frame(offset);
        }
    }

    pub fn add_handler(&mut self, h: Handler) {
        self.writing();
        self.handlers.push(h);
    }

    /// Binds the entry of an exception handler: the locals as they are, the exception on the stack.
    pub fn bind_handler(&mut self, l: Label, exception: VT) {
        self.writing();
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

    /// A branch instruction; the operands it pops are already gone from the model. It is written
    /// with a 16-bit offset, which `finish` widens where it does not reach.
    pub fn jump(&mut self, opcode: u8, target: Label) {
        self.before_insn();
        let at = self.pc();
        self.bytes.push(opcode);
        self.items.push(Item::Jump { at, target });
        self.bytes.extend_from_slice(&[0, 0]);
        if self.relax && opcode != op::GOTO {
            self.falls.push(Falls { jump: self.items.len() - 1, locals: self.locals.clone(), stack: self.stack.clone() });
        }
        self.record_target(target);
        if opcode == op::GOTO {
            self.reachable = false;
        }
    }

    /// A `tableswitch` or a `lookupswitch` as `layout` says, on the `Int` it pops: `targets[i]` the target of the
    /// layout's target `i`, `default` the others'. Its operands start at a multiple of four from the code's start, after
    /// zero bytes; every offset is a 32-bit one from the instruction (JVMS 6.5), and both follow the final layout
    /// (`Code::widen`). The path ends here.
    pub fn switch(&mut self, layout: &SwitchLayout, targets: &[Label], default: Label) {
        self.before_insn();
        self.pop();
        let at = self.pc();
        self.bytes.push(match layout {
            SwitchLayout::Table { .. } => op::TABLESWITCH,
            SwitchLayout::Lookup(_) => op::LOOKUPSWITCH,
        });
        let pad = switch_pad(at);
        self.bytes.extend(std::iter::repeat(0).take(pad as usize));
        let mut slots = Vec::new();
        self.offset_to(&mut slots, default);
        match layout {
            SwitchLayout::Table { low, high, slots: cases } => {
                self.bytes.extend_from_slice(&low.to_be_bytes());
                self.bytes.extend_from_slice(&high.to_be_bytes());
                for case in cases {
                    self.offset_to(&mut slots, case.map_or(default, |t| targets[t]));
                }
            }
            SwitchLayout::Lookup(pairs) => {
                self.bytes.extend_from_slice(&(pairs.len() as u32).to_be_bytes());
                for &(key, t) in pairs {
                    self.bytes.extend_from_slice(&key.to_be_bytes());
                    self.offset_to(&mut slots, targets[t]);
                }
            }
        }
        let end = self.pc();
        self.items.push(Item::Switch { at, pad, end, slots });
        self.reachable = false;
    }

    /// A switch's 32-bit offset to `target`, filled in when the code is finished.
    fn offset_to(&mut self, slots: &mut Vec<(u32, Label)>, target: Label) {
        slots.push((self.pc(), target));
        self.bytes.extend_from_slice(&[0, 0, 0, 0]);
        self.record_target(target);
    }

    /// Marks the end of a path: after a return or `athrow`.
    pub fn end_path(&mut self) {
        self.writing();
        self.reachable = false;
    }

    fn finish(mut self, cp: &mut ConstPool) -> (Vec<u8>, bool) {
        if self.bytes.is_empty() || !self.reachable && self.frames.last().map_or(false, |f| f.offset == self.pc()) {
            // A label bound at the very end needs an instruction to stand on.
            self.frames.retain(|f| f.offset < self.bytes.len() as u32 || self.bytes.is_empty());
        }
        self.lay_out();
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
        let overflow = self.overflow || code_len as usize > MAX_CODE;
        (out, overflow)
    }

    /// Writes each jump's and switch's offsets: a jump's in 16 bits where every one reaches, else, while the method
    /// is written again (`relax`), with the jumps that do not reach widened (`Code::widen`).
    fn lay_out(&mut self) {
        let label = |c: &Code, l: Label| c.labels[l.0 as usize].offset.expect("unbound label") as i64;
        for item in &self.items {
            match item {
                Item::Jump { at, target } => {
                    let delta = label(self, *target) - *at as i64;
                    if delta < i16::MIN as i64 || delta > i16::MAX as i64 {
                        if self.relax && self.bytes.len() <= MAX_CODE {
                            self.widen();
                        } else {
                            self.overflow = true;
                        }
                        return;
                    }
                    let p = *at as usize + 1;
                    self.bytes[p..p + 2].copy_from_slice(&(delta as i16).to_be_bytes());
                }
                Item::Switch { at, slots, .. } => {
                    for &(p, target) in slots {
                        let offset = (label(self, target) - *at as i64) as i32;
                        self.bytes[p as usize..p as usize + 4].copy_from_slice(&offset.to_be_bytes());
                    }
                }
            }
        }
    }

    /// Lays the code out again with the jumps that do not reach in 16 bits wide, as scalac's ASM
    /// writes them (`MethodWriter.visitJumpInsn`, and `ClassReader` for a forward one): a `goto`
    /// as a `goto_w`, a conditional jump as its inverse over a `goto_w`, whose 8 bytes the inverse
    /// jumps over to the code it falls through to. A widened jump moves what follows it, and a
    /// switch takes the padding of its new offset, so the layout is taken again until every narrow
    /// jump reaches; a jump once wide stays wide, as ASM's round trip keeps a `goto_w`. Every offset
    /// then follows the final layout: the labels (the jumps' and switches' targets, the handlers'
    /// ranges), the frames and the uninitialised types in them, the line table; the inverse of a
    /// widened conditional jump lands on a frame of the state it falls through with.
    fn widen(&mut self) {
        let n = self.items.len();
        let mut wide = vec![false; n];
        // moved[k]: what the items before item k add to its offset (a switch's padding can shrink it).
        let mut moved = vec![0i64; n + 1];
        let label = |c: &Code, l: Label| c.labels[l.0 as usize].offset.expect("unbound label");
        loop {
            let mut more = false;
            for k in 0..n {
                if let Item::Jump { at, target } = self.items[k] {
                    if wide[k] {
                        continue;
                    }
                    let delta = Self::moved_to(&self.items, &moved, label(self, target)) as i64 - (at as i64 + moved[k]);
                    if delta < i16::MIN as i64 || delta > i16::MAX as i64 {
                        wide[k] = true;
                        more = true;
                    }
                }
            }
            if !more {
                break;
            }
            for k in 0..n {
                let growth = match self.items[k] {
                    Item::Jump { at, .. } if wide[k] => if self.bytes[at as usize] == op::GOTO { 2 } else { 5 },
                    Item::Jump { .. } => 0,
                    Item::Switch { at, pad, .. } => switch_pad((at as i64 + moved[k]) as u32) as i64 - pad as i64,
                };
                moved[k + 1] = moved[k] + growth;
            }
        }
        let at = |o: u32| Self::moved_to(&self.items, &moved, o);
        let mut bytes = Vec::with_capacity((self.bytes.len() as i64 + moved[n]).max(0) as usize);
        let mut from = 0;
        for (k, item) in self.items.iter().enumerate() {
            bytes.extend_from_slice(&self.bytes[from..item.at() as usize]);
            let here = bytes.len() as i64;
            match item {
                Item::Jump { at: from_at, target } => {
                    from = *from_at as usize + 3;
                    let opcode = self.bytes[*from_at as usize];
                    let target = at(label(self, *target)) as i64;
                    if !wide[k] {
                        bytes.push(opcode);
                        bytes.extend_from_slice(&((target - here) as i16).to_be_bytes());
                    } else if opcode == op::GOTO {
                        bytes.push(op::GOTO_W);
                        bytes.extend_from_slice(&((target - here) as i32).to_be_bytes());
                    } else {
                        bytes.push(inverse(opcode));
                        bytes.extend_from_slice(&8u16.to_be_bytes());
                        bytes.push(op::GOTO_W);
                        bytes.extend_from_slice(&((target - here - 3) as i32).to_be_bytes());
                    }
                }
                Item::Switch { at: from_at, pad, end, slots } => {
                    from = *end as usize;
                    let operands = (*from_at + 1 + *pad) as usize;
                    bytes.push(self.bytes[*from_at as usize]);
                    bytes.extend(std::iter::repeat(0).take(switch_pad(here as u32) as usize));
                    let start = bytes.len();
                    bytes.extend_from_slice(&self.bytes[operands..*end as usize]);
                    for &(p, target) in slots {
                        let q = start + p as usize - operands;
                        bytes[q..q + 4].copy_from_slice(&((at(label(self, target)) as i64 - here) as i32).to_be_bytes());
                    }
                }
            }
        }
        bytes.extend_from_slice(&self.bytes[from..]);
        let place = |t: VT| match t {
            VT::Uninit(o) => VT::Uninit(at(o as u32) as u16),
            t => t,
        };
        let mut landings = self.falls.iter().filter(|f| wide[f.jump]).map(|f| Frame {
            offset: at(self.items[f.jump].at()) + 8,
            locals: f.locals.iter().map(|&t| place(t)).collect(),
            stack: f.stack.iter().map(|&t| place(t)).collect(),
        }).peekable();
        let mut frames = Vec::with_capacity(self.frames.len() + n);
        for f in &self.frames {
            let offset = at(f.offset);
            while let Some(l) = landings.next_if(|l| l.offset <= offset) {
                if l.offset < offset {
                    frames.push(l);
                }
            }
            frames.push(Frame { offset, locals: f.locals.iter().map(|&t| place(t)).collect(), stack: f.stack.iter().map(|&t| place(t)).collect() });
        }
        frames.extend(landings);
        for line in &mut self.lines {
            line.0 = at(line.0 as u32) as u16;
        }
        for label in &mut self.labels {
            if let Some(o) = label.offset {
                label.offset = Some(Self::moved_to(&self.items, &moved, o));
            }
        }
        self.frames = frames;
        self.bytes = bytes;
        self.overflow = self.bytes.len() > MAX_CODE;
    }

    /// Where the code at the first layout's offset `o` lands once the items before it grew or shrank.
    fn moved_to(items: &[Item], moved: &[i64], o: u32) -> u32 {
        (o as i64 + moved[items.partition_point(|i| i.at() < o)]) as u32
    }
}

/// The longest code a method may have: the JVM's `code_length` is below 65536.
const MAX_CODE: usize = 65535; // jdk: JVMS 4.7.3

/// The conditional jump taken where `opcode`'s is not.
fn inverse(opcode: u8) -> u8 {
    match opcode {
        op::IFNULL => op::IFNONNULL,
        op::IFNONNULL => op::IFNULL,
        o if (o - op::IFEQ) % 2 == 0 => o + 1,
        o => o - 1,
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

    /// A field declared before every other, as dotty's `LazyVals.addInFront` puts a lazy val's
    /// `VarHandle` before the template's statements.
    pub fn field_first(&mut self, access: u16, name: &str, desc: &str) {
        let name = self.cp.utf8(name);
        let desc = self.cp.utf8(desc);
        self.fields.insert(0, Member { access, name, desc, attrs: Vec::new() });
    }

    /// The class's interfaces in the place of the list it was made with: a list that holds the
    /// targets of the class's super calls is known when its bodies are written.
    pub fn set_interfaces(&mut self, interfaces: &[String]) {
        let list = interfaces.iter().map(|i| self.cp.class(i)).collect();
        self.interfaces = list;
    }

    /// Whether the class is `name` of these access flags, extending `super_name`.
    pub fn is_class(&self, access: u16, name: &str, super_name: &str) -> bool {
        self.access == access && self.class_index(name) == Some(self.this_class) && self.class_index(super_name) == Some(self.super_class)
    }

    /// Whether the class's interfaces hold `interfaces` in their order.
    pub fn holds_interfaces<'a>(&self, mut interfaces: impl Iterator<Item = &'a str>) -> bool {
        let mut own = self.interfaces.iter();
        interfaces.all(|n| self.class_index(n).is_some_and(|c| own.any(|&i| i == c)))
    }

    fn class_index(&self, name: &str) -> Option<u16> {
        self.cp.utf8.get(name).and_then(|&n| self.cp.consts.get(&Const::Class(n))).copied()
    }

    /// The first place where the class's methods are not `expected` in order, each by its
    /// access flags, name and descriptor; none where they are exactly those.
    pub fn methods_differ(&self, expected: &[(u16, &str, String)]) -> Option<usize> {
        self.differ(&self.methods, expected)
    }

    /// The same of its fields.
    pub fn fields_differ(&self, expected: &[(u16, &str, String)]) -> Option<usize> {
        self.differ(&self.fields, expected)
    }

    fn differ(&self, members: &[Member], expected: &[(u16, &str, String)]) -> Option<usize> {
        let same = |m: &Member, e: &(u16, &str, String)| m.access == e.0 && self.cp.utf8.get(e.1) == Some(&m.name) && self.cp.utf8.get(e.2.as_str()) == Some(&m.desc);
        let common = members.iter().zip(expected).position(|(m, e)| !same(m, e));
        common.or((members.len() != expected.len()).then_some(members.len().min(expected.len())))
    }

    /// The methods in the order `order` names them by name and signature, each once: the class plan's.
    pub fn order_methods<'n>(&mut self, order: impl Iterator<Item = (&'n str, &'n str)>) {
        let mut methods: Vec<Option<Member>> = std::mem::take(&mut self.methods).into_iter().map(Some).collect();
        for (name, desc) in order {
            let (n, d) = (self.cp.utf8.get(name).copied(), self.cp.utf8.get(desc).copied());
            if let Some(m) = methods.iter_mut().find(|m| m.as_ref().is_some_and(|m| Some(m.name) == n && Some(m.desc) == d)).and_then(Option::take) {
                self.methods.push(m);
            }
        }
        self.methods.extend(methods.into_iter().flatten());
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

    /// Adds to the method `name` `desc` written already what a forwarder writes of the method it
    /// forwards to, as dotty copies its annotations (`MixinOps.mkForwarderSym`) and the backend
    /// writes them (`BCodeSkelBuilder.initJMethod`): the exceptions it declares, its deprecation,
    /// its annotations and its parameters', runtime-visible and invisible, in ASM's order of the
    /// attributes. Each list in the reverse of the method's order, as `addAnnotations` prepends
    /// each annotation.
    pub fn method_metadata(&mut self, name: &str, desc: &str, from: &crate::classfile::Method) {
        let (Some(&n), Some(&d)) = (self.cp.utf8.get(name), self.cp.utf8.get(desc)) else { return };
        let Some(at) = self.methods.iter().position(|m| m.name == n && m.desc == d) else { return };
        let mut attrs = Vec::new();
        if !from.exceptions.is_empty() {
            let mut body = (from.exceptions.len() as u16).to_be_bytes().to_vec();
            for e in from.exceptions.iter().rev() {
                body.extend_from_slice(&self.cp.class(e).to_be_bytes());
            }
            attrs.push((self.cp.utf8("Exceptions"), body));
        }
        if from.deprecated {
            attrs.push((self.cp.utf8("Deprecated"), Vec::new()));
        }
        if let Some(kept) = &from.kept_annotations {
            for (attr, list) in [("RuntimeVisibleAnnotations", &kept.0), ("RuntimeInvisibleAnnotations", &kept.1)] {
                if list.is_empty() {
                    continue;
                }
                let mut body = (list.len() as u16).to_be_bytes().to_vec();
                for a in list.iter().rev() {
                    self.cp.annotation(a, &mut body);
                }
                attrs.push((self.cp.utf8(attr), body));
            }
        }
        if let Some(kept) = &from.kept_parameter_annotations {
            for (attr, lists) in [("RuntimeVisibleParameterAnnotations", &kept.0), ("RuntimeInvisibleParameterAnnotations", &kept.1)] {
                if lists.iter().all(|l| l.is_empty()) {
                    continue;
                }
                let mut body = vec![lists.len() as u8];
                for list in lists {
                    body.extend_from_slice(&(list.len() as u16).to_be_bytes());
                    for a in list.iter().rev() {
                        self.cp.annotation(a, &mut body);
                    }
                }
                attrs.push((self.cp.utf8(attr), body));
            }
        }
        self.methods[at].attrs.extend(attrs);
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
    /// The class file; the bound its constant pool passes instead, with the entries `finish` adds
    /// itself (`ConstPool::unwritable`).
    pub fn finish(mut self, output_version: u32) -> Result<Vec<u8>, PoolLimit> {
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
        if let Some(limit) = self.cp.unwritable() {
            return Err(limit);
        }
        out.extend_from_slice(&(self.cp.count as u16).to_be_bytes());
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
        Ok(out)
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
    pub const TABLESWITCH: u8 = 0xaa;
    pub const LOOKUPSWITCH: u8 = 0xab;
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
    pub const MONITORENTER: u8 = 0xc2;
    pub const MONITOREXIT: u8 = 0xc3;
    pub const CHECKCAST: u8 = 0xc0;
    pub const INSTANCEOF: u8 = 0xc1;
    pub const IFNULL: u8 = 0xc6;
    pub const IFNONNULL: u8 = 0xc7;
    pub const GOTO_W: u8 = 0xc8;
}

/// A jar is a zip archive; the entries are stored without compression. Its manifest names the
/// number of the products it holds (`products::JAR_ATTRIBUTE`), for whoever reads the jar's
/// provenance: a class path reads a jar as a library whatever number it names.
pub fn write_jar(path: &str, main_class: Option<&str>, classes: &[(String, Vec<u8>)]) -> std::io::Result<()> {
    let mut entries: Vec<(String, std::borrow::Cow<[u8]>)> = Vec::with_capacity(classes.len() + 1);
    let mut manifest = format!("Manifest-Version: 1.0\r\nCreated-By: teq\r\n{}: {}\r\n", crate::products::JAR_ATTRIBUTE, crate::products::ABI);
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

/// How a switch's keys are written (`Code::switch`), as dotty's back end chooses (`BCodeIdiomatic.emitSWITCH`,
/// 394-462 at 3.8.4, with `MIN_SWITCH_DENSITY` = 0.7, BCodeHelpers.scala 482).
#[derive(Debug, PartialEq)]
pub enum SwitchLayout {
    /// A `tableswitch` from the lowest key to the highest: for each value between them the index of its target, or
    /// none where no key stands (the default's).
    Table { low: i32, high: i32, slots: Vec<Option<usize>> },
    /// A `lookupswitch`: the keys in increasing order, each with the index of its target.
    Lookup(Vec<(i32, usize)>),
}

/// The layout of a switch whose keys are `keys`, each with the index of its target: the keys sorted; a table where they
/// fill at least 0.7 of the values from the lowest to the highest (the range counted in 64 bits), else a lookup; with
/// no key, a lookup of the default alone. A key that stands twice is refused, as `emitSWITCH` aborts.
pub fn switch_layout(keys: &[(i32, usize)]) -> SwitchLayout {
    let mut sorted = keys.to_vec();
    sorted.sort_by_key(|&(key, _)| key);
    assert!(sorted.windows(2).all(|w| w[0].0 != w[1].0), "a key in two places of a switch");
    if sorted.is_empty() {
        return SwitchLayout::Lookup(sorted);
    }
    let (low, high) = (sorted[0].0, sorted[sorted.len() - 1].0);
    let range = high as i64 - low as i64 + 1;
    if (sorted.len() as f64) / (range as f64) < 0.7 {
        return SwitchLayout::Lookup(sorted);
    }
    let mut slots = vec![None; range as usize];
    for &(key, target) in &sorted {
        slots[(key as i64 - low as i64) as usize] = Some(target);
    }
    SwitchLayout::Table { low, high, slots }
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

#[cfg(test)]
mod tests {
    use super::{op, switch_layout, Code, ConstPool, Handler, SwitchLayout, VT};

    /// The code of a `finish`ed method: its bytes, and whether it was refused.
    /// A code these tests write instructions into bare, as no node's: it opens inside the node scope every instruction
    /// is written in (`Code::before_insn`).
    fn bare(locals: Vec<VT>) -> Code {
        let mut c = Code::new(locals);
        c.in_node = 1;
        c
    }

    fn code_of(c: Code) -> (Vec<u8>, bool) {
        let (out, overflow) = c.finish(&mut ConstPool::new());
        let len = u32::from_be_bytes([out[4], out[5], out[6], out[7]]) as usize;
        (out[8..][..len].to_vec(), overflow)
    }

    fn filler(c: &mut Code, bytes: usize) {
        for _ in 0..bytes {
            c.op(op::NOP);
        }
    }

    /// `iload_0; if<cond> L; nop * gap; L: return`, the jump's offset 3 + gap.
    fn forward(gap: usize, relax: bool) -> (Vec<u8>, bool) {
        let mut c = bare(vec![VT::Int]);
        c.relax = relax;
        let l = c.new_label();
        c.var_insn(op::ILOAD, op::ILOAD_0, 0);
        c.jump(op::IFLE, l);
        filler(&mut c, gap);
        c.bind(l);
        c.op(op::RETURN);
        code_of(c)
    }

    fn short_at(code: &[u8], at: usize) -> i64 {
        i16::from_be_bytes([code[at], code[at + 1]]) as i64
    }

    fn int_at(code: &[u8], at: usize) -> i64 {
        i32::from_be_bytes([code[at], code[at + 1], code[at + 2], code[at + 3]]) as i64
    }

    #[test]
    fn a_forward_jump_is_widened_past_32767_alone() {
        let (code, overflow) = forward(32764, false);
        assert!(!overflow);
        assert_eq!((code[1], short_at(&code, 2)), (op::IFLE, 32767));
        let (_, overflow) = forward(32765, false);
        assert!(overflow, "outside `relax` a jump that does not reach makes the unit be written again");
        let (code, overflow) = forward(32765, true);
        assert!(!overflow);
        // The inverse over a `goto_w` to the label, which moved by the 5 bytes the jump grew.
        assert_eq!((code[1], short_at(&code, 2), code[4]), (op::IFGT, 8, op::GOTO_W));
        assert_eq!(int_at(&code, 5), 1 + 8 + 32765 - 4);
        assert_eq!(code.len(), 1 + 8 + 32765 + 1);
        let (code, _) = forward(32764, true);
        assert_eq!((code[1], short_at(&code, 2), code.len()), (op::IFLE, 32767, 1 + 3 + 32764 + 1));
    }

    /// `L: nop * gap; goto L; return`, the jump's offset -gap.
    fn backward(gap: usize) -> Vec<u8> {
        let mut c = bare(vec![]);
        c.relax = true;
        let l = c.new_label();
        c.bind(l);
        filler(&mut c, gap);
        c.jump(op::GOTO, l);
        c.op(op::RETURN);
        code_of(c).0
    }

    #[test]
    fn a_backward_jump_is_widened_past_minus_32768_alone() {
        let code = backward(32768);
        assert_eq!((code[32768], short_at(&code, 32769), code.len()), (op::GOTO, -32768, 32768 + 3 + 1));
        let code = backward(32769);
        assert_eq!((code[32769], int_at(&code, 32770), code.len()), (op::GOTO_W, -32769, 32769 + 5 + 1));
    }

    /// A jump that reaches in the 16-bit layout is widened where the jumps widened inside its
    /// span push its target out of reach, and one whose span no wide jump is in stays narrow.
    #[test]
    fn a_jump_is_widened_where_the_widened_ones_push_it_out_of_reach() {
        let mut c = bare(vec![VT::Int]);
        c.relax = true;
        let (near, far, after) = (c.new_label(), c.new_label(), c.new_label());
        c.var_insn(op::ILOAD, op::ILOAD_0, 0);
        c.jump(op::IFEQ, near); // at 1: 32767 to `near` in the 16-bit layout
        c.jump(op::GOTO, far); // at 4: past 32767 to `far`
        filler(&mut c, 32767 - 6);
        c.bind(near);
        c.var_insn(op::ILOAD, op::ILOAD_0, 0);
        c.jump(op::IFNE, after); // reaches: no wide jump between it and its target
        filler(&mut c, 10);
        c.bind(after);
        filler(&mut c, 40);
        c.bind(far);
        c.op(op::RETURN);
        let (code, overflow) = code_of(c);
        assert!(!overflow);
        assert_eq!((code[1], code[4], code[9]), (op::IFNE, op::GOTO_W, op::GOTO_W), "the first widened by the second's growth");
        let near_at = 32768 + 7;
        assert_eq!(code[near_at + 1], op::IFNE);
        assert_eq!(short_at(&code, near_at + 2), 3 + 10);
        assert_eq!(int_at(&code, 5), near_at as i64 - 4);
        assert_eq!(int_at(&code, 10), (near_at + 1 + 3 + 10 + 40) as i64 - 9);
    }

    /// The frames, the uninitialised types in them, the handlers' ranges and the line table follow
    /// the widened layout, and the widened jump's inverse lands on a frame of the state it falls
    /// through with.
    #[test]
    fn every_offset_follows_the_widened_layout() {
        let mut c = bare(vec![VT::Int]);
        c.relax = true;
        let (skip, start, end, handler) = (c.new_label(), c.new_label(), c.new_label(), c.new_label());
        c.var_insn(op::ILOAD, op::ILOAD_0, 0);
        c.jump(op::IFLE, skip); // at 1, widened
        c.new_local(VT::Int);
        c.op(op::ICONST_0);
        c.var_insn(op::ISTORE, op::ISTORE_0, 1); // a local the code after the jump has
        c.line(7);
        c.bind(start);
        let new_at = c.pc();
        c.op_u16(op::NEW, 1);
        c.push(VT::Uninit(new_at as u16));
        let mid = c.new_label();
        c.jump(op::GOTO, mid);
        c.bind(mid);
        c.op(op::POP);
        c.pop();
        c.bind(end);
        filler(&mut c, 32770);
        c.op(op::RETURN);
        c.end_path();
        c.bind_handler(handler, VT::Object(2));
        c.op(op::ATHROW);
        c.end_path();
        c.add_handler(Handler { start, end, handler, catch_type: 0 });
        c.bind(skip);
        c.op(op::RETURN);
        c.lay_out();
        assert_eq!((c.bytes[1], c.bytes[4]), (op::IFGT, op::GOTO_W));
        let at = |o: u32| o + 5;
        let offsets: Vec<u32> = c.frames.iter().map(|f| f.offset).collect();
        assert_eq!(offsets[0], 9, "the inverse's landing");
        let landing = &c.frames[0];
        assert_eq!((landing.locals.clone(), landing.stack.clone()), (vec![VT::Int], vec![]));
        let mid_frame = c.frames.iter().find(|f| f.offset == at(new_at + 3 + 3)).expect("the label after the `new`'s frame");
        assert_eq!(mid_frame.stack, vec![VT::Uninit(at(new_at) as u16)]);
        assert_eq!(c.lines, vec![(at(new_at) as u16, 7)]);
        assert_eq!(c.labels[start.0 as usize].offset, Some(at(new_at)));
        assert_eq!(c.labels[handler.0 as usize].offset, Some(at(new_at + 7 + 32770 + 1)));
        assert_eq!(c.labels[skip.0 as usize].offset, Some(at(new_at + 7 + 32770 + 2)));
    }

    use super::{ClassWriter, PoolLimit, ACC_PUBLIC, ACC_SUPER, MAX_POOL_COUNT, MAX_TEXT_BYTES};

    /// A class writer whose pool holds `count - 1` entries: its own two classes, then texts.
    fn pool_of(count: u32) -> ClassWriter {
        let mut cw = ClassWriter::new(ACC_PUBLIC | ACC_SUPER, "P", "java/lang/Object", &[]);
        let mut i = 0;
        while cw.cp.count < count {
            cw.cp.utf8(&format!("t{i}"));
            i += 1;
        }
        assert_eq!(cw.cp.count, count);
        cw
    }

    /// The pool's bound (JVMS 4.1): 65,534 entries, at the indexes 1 to 65,534, make a class file,
    /// which reads back with its `constant_pool_count` of 65,535; 65,535 and 65,536 entries do not,
    /// nor a long whose second slot would be at 65,535.
    #[test]
    fn the_constant_pool_holds_65534_entries() {
        let full = pool_of(MAX_POOL_COUNT).finish(0).expect("a pool of 65,534 entries is a class file's");
        assert_eq!(u16::from_be_bytes([full[8], full[9]]) as u32, MAX_POOL_COUNT);
        crate::classfile::ClassFile::parse(&full).expect("the class reads back");
        let mut past = pool_of(MAX_POOL_COUNT);
        past.cp.utf8("one more");
        past.cp.utf8("and one after it");
        assert!(past.cp.too_large(), "65,536 entries");
        let mut over = pool_of(MAX_POOL_COUNT);
        over.cp.utf8("one more");
        assert!(over.cp.too_large());
        assert_eq!(over.finish(0).err(), Some(PoolLimit::Entries), "an entry at 65,535");
        let mut long_fits = pool_of(MAX_POOL_COUNT - 2);
        long_fits.cp.long(7);
        assert!(long_fits.finish(0).is_ok(), "a long at 65,533 and 65,534");
        let mut long_over = pool_of(MAX_POOL_COUNT - 1);
        long_over.cp.long(7);
        assert_eq!(long_over.finish(0).err(), Some(PoolLimit::Entries), "a long at 65,534 and 65,535");
    }

    /// A method's parameters take at most 254 slots (dotty's `MaximumJvmParameters`), a long and a double two:
    /// the counts on each side of the bound.
    #[test]
    fn a_parameter_list_takes_254_slots() {
        use crate::jvm::names::{parameter_slots, MAX_PARAMETER_SLOTS};
        let desc = |params: &str| format!("({params})V");
        assert_eq!(parameter_slots(&desc(&"I".repeat(254))), (254, 254));
        assert_eq!(parameter_slots(&desc(&"I".repeat(255))), (255, 255));
        assert_eq!(parameter_slots(&desc(&"J".repeat(127))), (254, 127));
        assert_eq!(parameter_slots(&desc(&format!("{}I", "J".repeat(127)))), (255, 128));
        assert_eq!(parameter_slots(&desc(&format!("{}D", "Ljava/lang/String;".repeat(252)))), (254, 253));
        assert_eq!(MAX_PARAMETER_SLOTS, 254);
    }

    /// A text constant holds 65,535 bytes of modified UTF-8: one more is no class file's, and
    /// dotty's count of a text's bytes is the encoding's below the surrogates (`constant_text_fits`).
    #[test]
    fn a_text_constant_holds_65535_bytes() {
        let mut fits = ClassWriter::new(ACC_PUBLIC | ACC_SUPER, "P", "java/lang/Object", &[]);
        fits.cp.utf8(&"a".repeat(MAX_TEXT_BYTES));
        assert!(fits.finish(0).is_ok());
        let mut over = ClassWriter::new(ACC_PUBLIC | ACC_SUPER, "P", "java/lang/Object", &[]);
        over.cp.utf8(&format!("{}\u{e9}", "a".repeat(MAX_TEXT_BYTES - 1)));
        assert_eq!(over.finish(0).err(), Some(PoolLimit::Text));
        for text in ["a".repeat(MAX_TEXT_BYTES), "\u{e9}".repeat(MAX_TEXT_BYTES / 2), "\u{20ac}".repeat(MAX_TEXT_BYTES / 3), "\0".repeat(MAX_TEXT_BYTES / 2)] {
            assert!(super::constant_text_fits(&text, ""), "{} bytes", super::modified_utf8(&text).len());
            assert!(super::modified_utf8(&text).len() <= MAX_TEXT_BYTES);
            let longer = format!("{text}\u{20ac}");
            assert!(!super::constant_text_fits(&longer, ""), "one more character");
        }
        assert!(!super::constant_text_fits(&"a".repeat(40_000), &"b".repeat(30_000)), "a name and a descriptor counted together");
    }

    /// A switch's layout at the edges of `emitSWITCH`'s rule: no key, one key, the lowest and the highest `Int`
    /// together (a range of 2^32, counted in 64 bits), a density of exactly 0.7 (a table) and just under it (a
    /// lookup), a hole at each end of a table's range, and keys given out of order.
    #[test]
    fn a_switch_is_laid_out_as_dotty_lays_it_out() {
        assert_eq!(switch_layout(&[]), SwitchLayout::Lookup(vec![]));
        assert_eq!(switch_layout(&[(7, 0)]), SwitchLayout::Table { low: 7, high: 7, slots: vec![Some(0)] });
        assert_eq!(switch_layout(&[(i32::MAX, 1), (i32::MIN, 0)]), SwitchLayout::Lookup(vec![(i32::MIN, 0), (i32::MAX, 1)]));
        assert_eq!(switch_layout(&[(i32::MAX - 1, 0), (i32::MAX, 1)]), SwitchLayout::Table { low: i32::MAX - 1, high: i32::MAX, slots: vec![Some(0), Some(1)] });
        // Seven keys over ten values: 0.7, a table whose holes are the default's.
        let seven: Vec<(i32, usize)> = [0, 1, 2, 4, 5, 7, 9].iter().enumerate().map(|(t, &k)| (k, t)).collect();
        let SwitchLayout::Table { low: 0, high: 9, slots } = switch_layout(&seven) else { panic!("seven keys over ten values made no table") };
        assert_eq!(slots, vec![Some(0), Some(1), Some(2), None, Some(3), Some(4), None, Some(5), None, Some(6)]);
        // Six keys over nine values: 0.67, a lookup.
        assert!(matches!(switch_layout(&[(0, 0), (1, 1), (3, 2), (5, 3), (6, 4), (8, 5)]), SwitchLayout::Lookup(_)));
        // A hole next to each end of the range, the default's; the range is the keys' own, given in any order.
        let holes: Vec<(i32, usize)> = [9, 0, 2, 3, 4, 5, 6, 7].iter().enumerate().map(|(t, &k)| (k, t)).collect();
        assert_eq!(switch_layout(&holes), SwitchLayout::Table { low: 0, high: 9, slots: vec![Some(1), None, Some(2), Some(3), Some(4), Some(5), Some(6), Some(7), None, Some(0)] });
        assert!(std::panic::catch_unwind(|| switch_layout(&[(1, 0), (1, 1)])).is_err(), "a key twice made a layout");
    }

    /// `Code::switch` writes a layout as JVMS 6.5 has it, from each of the four phases of its opcode's pc: zero bytes up
    /// to a multiple of four from the code's start, the default's offset, then a table's low, high and one offset a
    /// value (a hole the default's, two keys of one case the same target) or a lookup's count and pairs, every offset
    /// from the opcode to its label; and a frame at each target.
    #[test]
    fn a_switch_is_written_aligned_with_its_offsets() {
        let word = |b: &[u8], at: usize| i32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
        for phase in 0..4usize {
            for table in [true, false] {
                let mut code = bare(Vec::new());
                for _ in 0..phase {
                    code.op(op::NOP);
                }
                code.op(op::ICONST_0);
                code.push(VT::Int);
                let (a, b, default) = (code.new_label(), code.new_label(), code.new_label());
                let keys: &[(i32, usize)] = if table { &[(1, 0), (2, 1), (4, 0)] } else { &[(-5, 0), (100, 1)] };
                let layout = switch_layout(keys);
                assert_eq!(matches!(layout, SwitchLayout::Table { .. }), table);
                code.switch(&layout, &[a, b], default);
                let mut at_label = Vec::new();
                for label in [a, b, default] {
                    code.bind(label);
                    at_label.push(code.pc() as i32);
                    code.op(op::RETURN);
                    code.end_path();
                }
                assert!(at_label.iter().all(|&pc| code.frames.iter().any(|f| f.offset as i32 == pc)), "a target with no frame");
                let (attr, _) = code.finish(&mut ConstPool::default());
                let bytes = &attr[8..];
                let at = phase + 1;
                assert_eq!(bytes[at], if table { op::TABLESWITCH } else { op::LOOKUPSWITCH });
                let start = (at + 1 + 3) / 4 * 4;
                assert!(bytes[at + 1..start].iter().all(|&z| z == 0), "padding of phase {phase}");
                let (pa, pb, pd) = (at_label[0] - at as i32, at_label[1] - at as i32, at_label[2] - at as i32);
                assert_eq!(word(bytes, start), pd);
                if table {
                    assert_eq!((word(bytes, start + 4), word(bytes, start + 8)), (1, 4));
                    let slots: Vec<i32> = [0, 1, 2, 3].iter().map(|&i| word(bytes, start + 12 + 4 * i)).collect();
                    assert_eq!(slots, vec![pa, pb, pd, pa]);
                } else {
                    assert_eq!(word(bytes, start + 4), 2);
                    let pairs: Vec<(i32, i32)> = (0..2).map(|i| (word(bytes, start + 8 + 8 * i), word(bytes, start + 12 + 8 * i))).collect();
                    assert_eq!(pairs, vec![(-5, pa), (100, pb)]);
                }
            }
        }
    }

    /// A switch behind a widened jump takes the padding of its new offset, and its offsets follow the final layout:
    /// the jump at 1 grows by 5, the switch moves from 5 to 10 and its padding from 2 bytes to 1.
    #[test]
    fn a_switch_after_a_widened_jump_is_padded_and_aimed_by_the_final_layout() {
        let mut c = bare(vec![VT::Int]);
        c.relax = true;
        let (far, a, b, d) = (c.new_label(), c.new_label(), c.new_label(), c.new_label());
        c.var_insn(op::ILOAD, op::ILOAD_0, 0);
        c.jump(op::IFLE, far);
        c.op(op::ICONST_0);
        c.push(VT::Int);
        let layout = switch_layout(&[(1, 0), (2, 1)]);
        c.switch(&layout, &[a, b], d);
        for l in [a, b, d] {
            c.bind(l);
            c.op(op::RETURN);
            c.end_path();
        }
        filler(&mut c, 32770);
        c.bind(far);
        c.op(op::RETURN);
        c.lay_out();
        let code = &c.bytes;
        assert_eq!((code[1], code[4]), (op::IFGT, op::GOTO_W));
        assert_eq!(int_at(code, 5), 31 + 32770 + 4 - 4);
        assert_eq!((code[10], code[11]), (op::TABLESWITCH, 0));
        let words: Vec<i64> = [0, 1, 2, 3, 4].iter().map(|&i| int_at(code, 12 + 4 * i)).collect();
        assert_eq!(words, vec![34 - 10, 1, 2, 32 - 10, 33 - 10]);
        assert!([32, 33, 34].iter().all(|&o| c.frames.iter().any(|f| f.offset == o)), "a target's frame not moved");
        assert_eq!(code.len(), 31 + 32770 + 1 + 4);
    }

    /// The jar writer's CRC-32 is the one zip.rs reads jars by: the check value, and every length
    /// past the slicing by 8's blocks.
    #[test]
    fn the_jars_crc32_is_zips() {
        let table = super::crc_table();
        assert_eq!(super::crc32(&table, b"123456789"), 0xcbf4_3926);
        let data: Vec<u8> = (0..300u32).map(|i| (i * 131 % 256) as u8).collect();
        for len in 0..data.len() {
            assert_eq!(super::crc32(&table, &data[..len]), crate::zip::crc32(&data[..len]), "length {len}");
        }
    }
}
