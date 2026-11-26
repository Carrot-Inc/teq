//! The reader of a NIR file, after `nir.serialization.BinaryDeserializer` of Scala Native 0.5.12:
//! a prelude of the sections' offsets, a table from each definition's name to its place, and
//! sections of strings, positions, symbols, types, definitions, values and instruction lists
//! that refer to each other by offset. A definition is decoded when asked for, so a link pays
//! for the bodies it reaches only.

use super::*;

pub const MAGIC: u32 = 0x2e4e_4952;
pub const COMPAT: u32 = 6;
pub const REVISION: u32 = 11;
/// The tag of a method with a body among a definition's.
const DEFINE_TAG: u8 = 4;
pub const PRELUDE_LEN: usize = 45;
/// How deep types, values, successors and link-time conditions nest, one inside the other, before
/// a file is refused. The format has no limit and Scala Native's reader (0.5.12) has none of its
/// own: it overflows its thread's stack, which at 1 MB is between 1,000 and 1,250 nested types or
/// values and between 6,000 and 7,000 nested successors or conditions. This reader recurses too,
/// and the bound keeps it within a fraction of an ordinary thread's stack. Scala Native's own
/// libraries nest seven deep at most.
const MAX_DEPTH: u32 = 1024; // where a JDK thread's default 1 MB stack ends Scala Native's reader on types and values

type Res<T> = Result<T, String>;

#[derive(Clone, Copy)]
struct Sections {
    offsets: u32,
    strings: u32,
    positions: u32,
    globals: u32,
    types: u32,
    defns: u32,
    vals: u32,
    insts: u32,
}

/// Whether a file's prelude says the file holds an entry point (a static initializer or an
/// exported function); none for what is no prelude of a NIR file this reader takes.
pub fn entry_points_flag(start: &[u8]) -> Option<bool> {
    let word = |i: usize| u32::from_be_bytes(start[i * 4..i * 4 + 4].try_into().unwrap());
    (start.len() >= PRELUDE_LEN && word(0) == MAGIC && word(1) == COMPAT && word(2) <= REVISION).then(|| start[PRELUDE_LEN - 1] != 0)
}

/// One file's bytes with what has been decoded of its shared sections, by place, as
/// `BinaryDeserializer`'s cache holds it: an entry is decoded once, whichever definition names it.
/// A type or a value is kept with the levels it holds, its own and those of what it is made of,
/// which a reference to it met deeper adds to the depth it is met at.
pub struct NirFile<'a> {
    buf: &'a [u8],
    /// The revision of the format the file was written in, which an earlier Scala Native's is.
    pub revision: u32,
    sections: Sections,
    strings: FxMap<u32, Str>,
    globals: FxMap<u32, Global>,
    types: FxMap<u32, (Ty, u32)>,
    vals: FxMap<usize, (Val, u32)>,
    positions: FxMap<u32, Pos>,
}

impl<'a> NirFile<'a> {
    pub fn open(buf: &'a [u8]) -> Res<NirFile<'a>> {
        if buf.len() < PRELUDE_LEN {
            return Err("no NIR file: shorter than a prelude".to_string());
        }
        let word = |i: usize| u32::from_be_bytes(buf[i * 4..i * 4 + 4].try_into().unwrap());
        if word(0) != MAGIC {
            return Err("no NIR file: its first bytes are not .NIR".to_string());
        }
        let (compat, revision) = (word(1), word(2));
        if compat != COMPAT || revision > REVISION {
            return Err(format!("NIR {}.{}, where {}.0 to {}.{} are read", compat, revision, COMPAT, COMPAT, REVISION));
        }
        let sections = Sections {
            offsets: word(3),
            strings: word(4),
            positions: word(5),
            globals: word(6),
            types: word(7),
            defns: word(8),
            vals: word(9),
            insts: word(10),
        };
        Ok(NirFile {
            buf,
            revision,
            sections,
            strings: FxMap::default(),
            globals: FxMap::default(),
            types: FxMap::default(),
            vals: FxMap::default(),
            positions: FxMap::default(),
        })
    }

    /// The table as the file has it: the name and the offset in the definitions' section of every
    /// definition written, in the order they were written in.
    pub fn table(&mut self, cx: &mut Cx) -> Res<Vec<(Global, u32)>> {
        let pos = self.sections.offsets as usize;
        let mut r = Reader::new(self, cx, pos);
        let mut entries: Vec<(Global, u32)> = Vec::new();
        loop {
            let global = r.global()?;
            let offset = r.signed_int()? as u32;
            if global == Global::NONE {
                return Ok(entries);
            }
            entries.push((global, offset));
        }
    }

    /// Every definition's name with its offset in the definitions' section, in the file's order.
    /// The plugin writes some names twice (a bridge beside the method it stands for): the table's
    /// last entry of a name is the definition, as in the map Scala Native reads the table into.
    pub fn offsets(&mut self, cx: &mut Cx) -> Res<Vec<(Global, u32)>> {
        let mut entries: Vec<(Global, u32)> = Vec::new();
        let mut index: FxMap<Global, usize> = FxMap::default();
        for (global, offset) in self.table(cx)? {
            match index.get(&global) {
                Some(&at) => entries[at].1 = offset,
                None => {
                    index.insert(global, entries.len());
                    entries.push((global, offset));
                }
            }
        }
        Ok(entries)
    }

    pub fn defn(&mut self, cx: &mut Cx, offset: u32) -> Res<Defn> {
        let pos = self.sections.defns as usize + offset as usize;
        Reader::new(self, cx, pos).defn()
    }

    /// Whether a definition is a method with a body, and its attributes: what a link asks of
    /// every definition of a class it loads, the bodies left undecoded.
    pub fn defn_head(&mut self, cx: &mut Cx, offset: u32) -> Res<(bool, Attrs)> {
        let pos = self.sections.defns as usize + offset as usize;
        let mut r = Reader::new(self, cx, pos);
        let tag = r.byte()?;
        r.global()?;
        Ok((tag == DEFINE_TAG, r.attrs()?))
    }

    pub fn defns(&mut self, cx: &mut Cx) -> Res<Vec<Defn>> {
        let offsets = self.offsets(cx)?;
        self.defns_at(cx, offsets)
    }

    /// Every definition written, in the file's order: a name's every definition, where `defns`
    /// has its last.
    pub fn written_defns(&mut self, cx: &mut Cx) -> Res<Vec<Defn>> {
        let table = self.table(cx)?;
        self.defns_at(cx, table)
    }

    fn defns_at(&mut self, cx: &mut Cx, offsets: Vec<(Global, u32)>) -> Res<Vec<Defn>> {
        let mut defns = Vec::with_capacity(offsets.len());
        for (name, offset) in offsets {
            let defn = self.defn(cx, offset).map_err(|e| {
                let mut mangled = Vec::new();
                cx.mangle(name, &mut mangled);
                format!("{}: {}", String::from_utf8_lossy(&mangled), e)
            })?;
            defns.push(defn);
        }
        Ok(defns)
    }
}

struct Reader<'r, 'a, 'c> {
    file: &'r mut NirFile<'a>,
    cx: &'c mut Cx,
    pos: usize,
    depth: u32,
    /// The deepest level the type or the value being decoded has reached.
    reached: u32,
    /// The places of the types and of the values being decoded: a damaged file's offsets can name
    /// each other in a circle, and a type or a value met again inside itself is refused.
    open_types: FxMap<u32, ()>,
    open_vals: FxMap<usize, ()>,
}

impl<'r, 'a, 'c> Reader<'r, 'a, 'c> {
    fn new(file: &'r mut NirFile<'a>, cx: &'c mut Cx, pos: usize) -> Self {
        Reader { file, cx, pos, depth: 0, reached: 0, open_types: FxMap::default(), open_vals: FxMap::default() }
    }

    /// One level deeper, which the caller leaves again by counting `depth` down.
    fn enter(&mut self) -> Res<()> {
        if self.depth == MAX_DEPTH {
            return Err(format!("nested deeper than {}", MAX_DEPTH));
        }
        self.depth += 1;
        self.reached = self.reached.max(self.depth);
        Ok(())
    }

    /// What `read` decodes one level deeper, with the levels it holds.
    fn nest<T>(&mut self, read: impl FnOnce(&mut Self) -> Res<T>) -> Res<(T, u32)> {
        let start = self.depth;
        let outer = std::mem::replace(&mut self.reached, start);
        self.enter()?;
        let decoded = read(self)?;
        self.depth -= 1;
        let levels = self.reached - start;
        self.reached = self.reached.max(outer);
        Ok((decoded, levels))
    }

    /// A type or a value decoded before, of `levels`, named again here.
    fn shared(&mut self, levels: u32) -> Res<()> {
        if self.depth + levels > MAX_DEPTH {
            return Err(format!("nested deeper than {}", MAX_DEPTH));
        }
        self.reached = self.reached.max(self.depth + levels);
        Ok(())
    }

    fn byte(&mut self) -> Res<u8> {
        let b = *self.file.buf.get(self.pos).ok_or("the file ends within a definition")?;
        self.pos += 1;
        Ok(b)
    }

    fn take(&mut self, n: usize) -> Res<&[u8]> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.file.buf.len()).ok_or("the file ends within a definition")?;
        let bytes = &self.file.buf[self.pos..end];
        self.pos = end;
        Ok(bytes)
    }

    fn array<const N: usize>(&mut self) -> Res<[u8; N]> {
        let mut bytes = [0; N];
        bytes.copy_from_slice(self.take(N)?);
        Ok(bytes)
    }

    fn bool(&mut self) -> Res<bool> {
        Ok(self.byte()? != 0)
    }

    fn unsigned_int(&mut self) -> Res<u32> {
        let (mut result, mut shift) = (0u32, 0);
        loop {
            let b = self.byte()?;
            result |= ((b & 0x7f) as u32) << shift;
            shift += 7;
            if b & 0x80 == 0 {
                return Ok(result);
            }
            if shift == 35 {
                return Err("a LEB128 integer longer than five bytes".to_string());
            }
        }
    }

    fn signed_int(&mut self) -> Res<i32> {
        let (mut result, mut shift) = (0u32, 0);
        loop {
            let b = self.byte()?;
            result |= ((b & 0x7f) as u32) << shift;
            shift += 7;
            if b & 0x80 == 0 {
                if shift < 32 && b & 0x40 != 0 {
                    result |= u32::MAX << shift;
                }
                return Ok(result as i32);
            }
            if shift == 35 {
                return Err("a LEB128 integer longer than five bytes".to_string());
            }
        }
    }

    fn unsigned_long(&mut self) -> Res<u64> {
        let (mut result, mut shift) = (0u64, 0);
        loop {
            let b = self.byte()?;
            result |= ((b & 0x7f) as u64) << shift;
            shift += 7;
            if b & 0x80 == 0 {
                return Ok(result);
            }
            if shift == 70 {
                return Err("a LEB128 integer longer than ten bytes".to_string());
            }
        }
    }

    fn signed_long(&mut self) -> Res<i64> {
        let (mut result, mut shift) = (0u64, 0);
        loop {
            let b = self.byte()?;
            result |= ((b & 0x7f) as u64) << shift;
            shift += 7;
            if b & 0x80 == 0 {
                if shift < 64 && b & 0x40 != 0 {
                    result |= u64::MAX << shift;
                }
                return Ok(result as i64);
            }
            if shift == 70 {
                return Err("a LEB128 integer longer than ten bytes".to_string());
            }
        }
    }

    /// A count of elements that each take a byte at least: one beyond the file's end is refused
    /// before anything is allocated for it.
    fn count(&mut self) -> Res<usize> {
        let n = self.unsigned_int()? as usize;
        if n > self.file.buf.len() {
            return Err("a sequence longer than the file".to_string());
        }
        Ok(n)
    }

    fn seq<T>(&mut self, mut item: impl FnMut(&mut Self) -> Res<T>) -> Res<Box<[T]>> {
        let n = self.count()?;
        let mut items = Vec::with_capacity(n);
        for _ in 0..n {
            items.push(item(self)?);
        }
        Ok(items.into_boxed_slice())
    }

    fn opt<T>(&mut self, item: impl FnOnce(&mut Self) -> Res<T>) -> Res<Option<T>> {
        if self.byte()? == 0 {
            Ok(None)
        } else {
            item(self).map(Some)
        }
    }

    /// What `read` decodes at the place an offset into the section at `start` names, the reader
    /// back after the offset.
    fn at<T>(&mut self, start: u32, read: impl FnOnce(&mut Self) -> Res<T>) -> Res<T> {
        let target = start as usize + self.unsigned_int()? as usize;
        let back = std::mem::replace(&mut self.pos, target);
        let value = read(self);
        self.pos = back;
        value
    }

    fn target(&mut self, start: u32) -> Res<u32> {
        Ok(start.wrapping_add(self.unsigned_int()?))
    }

    fn string(&mut self) -> Res<Str> {
        let target = self.target(self.file.sections.strings)?;
        if let Some(&s) = self.file.strings.get(&target) {
            return Ok(s);
        }
        let back = std::mem::replace(&mut self.pos, target as usize);
        let n = self.count()?;
        let mut units = Vec::with_capacity(n);
        for _ in 0..n {
            units.push(self.unsigned_int()? as u16);
        }
        self.pos = back;
        let s = self.cx.str(&wtf8(&units));
        self.file.strings.insert(target, s);
        Ok(s)
    }

    fn global(&mut self) -> Res<Global> {
        let target = self.target(self.file.sections.globals)?;
        if let Some(&g) = self.file.globals.get(&target) {
            return Ok(g);
        }
        let back = std::mem::replace(&mut self.pos, target as usize);
        let data = match self.byte()? {
            1 => GlobalData::None,
            2 => GlobalData::Top(self.string()?),
            3 => GlobalData::Member(self.owner()?, self.string()?),
            tag => return Err(format!("tag {} of a symbol", tag)),
        };
        self.pos = back;
        let g = self.cx.global(data);
        self.file.globals.insert(target, g);
        Ok(g)
    }

    /// A member's owner, read as the class it has to be and not as any symbol: a damaged file's
    /// member can name itself as its owner.
    fn owner(&mut self) -> Res<Global> {
        let target = self.target(self.file.sections.globals)?;
        let owner = match self.file.globals.get(&target) {
            Some(&g) => g,
            None => {
                let back = std::mem::replace(&mut self.pos, target as usize);
                if self.byte()? != 2 {
                    return Err("a member whose owner is no class".to_string());
                }
                let top = GlobalData::Top(self.string()?);
                self.pos = back;
                let g = self.cx.global(top);
                self.file.globals.insert(target, g);
                g
            }
        };
        if !matches!(self.cx.global_data(owner), GlobalData::Top(_)) {
            return Err("a member whose owner is no class".to_string());
        }
        Ok(owner)
    }

    fn globals(&mut self) -> Res<Box<[Global]>> {
        self.seq(Self::global)
    }

    fn ty(&mut self) -> Res<Ty> {
        let target = self.target(self.file.sections.types)?;
        if let Some(&(t, levels)) = self.file.types.get(&target) {
            self.shared(levels)?;
            return Ok(t);
        }
        if self.open_types.insert(target, ()).is_some() {
            return Err("a type that holds itself".to_string());
        }
        let back = std::mem::replace(&mut self.pos, target as usize);
        let (ty, levels) = self.nest(|r| {
            Ok(match r.byte()? {
                1 => Type::Vararg,
                2 => Type::Bool,
                3 => Type::Ptr,
                4 => Type::Char,
                5 => Type::Byte,
                6 => Type::Short,
                7 => Type::Int,
                8 => Type::Long,
                9 => Type::Float,
                10 => Type::Double,
                11 => Type::ArrayValue(r.ty()?, r.unsigned_int()? as i32),
                12 => Type::StructValue(r.types()?),
                13 => Type::Function(r.types()?, r.ty()?),
                14 => Type::Null,
                15 => Type::Nothing,
                16 => Type::Virtual,
                17 => Type::Var(r.ty()?),
                18 => Type::Unit,
                19 => Type::Array { of: r.ty()?, nullable: r.bool()? },
                20 => Type::Ref { name: r.global()?, exact: r.bool()?, nullable: r.bool()? },
                21 => Type::Size,
                22 => Type::Int128,
                tag => return Err(format!("tag {} of a type", tag)),
            })
        })?;
        self.pos = back;
        self.open_types.remove(&target);
        let t = self.cx.ty(ty);
        self.file.types.insert(target, (t, levels));
        Ok(t)
    }

    fn types(&mut self) -> Res<Box<[Ty]>> {
        self.seq(Self::ty)
    }

    fn position(&mut self) -> Res<Pos> {
        let place = self.unsigned_int()?;
        // An offset is a JVM `Int` that is not negative, as Scala Native writes and reads it, so that
        // a place counted from one is a number of 32 bits.
        if place > i32::MAX as u32 {
            return Err("a position beyond the 31 bits of an offset".to_string());
        }
        let target = self.file.sections.positions.wrapping_add(place);
        if let Some(&p) = self.file.positions.get(&target) {
            return Ok(p);
        }
        let back = std::mem::replace(&mut self.pos, target as usize);
        let file = self.string()?;
        let place = if file == Str::EMPTY { NonZeroU32::new(place + 1) } else { None };
        let p = Pos { file, line: self.unsigned_int()?, column: self.unsigned_int()?, place };
        self.pos = back;
        self.file.positions.insert(target, p);
        Ok(p)
    }

    fn local(&mut self) -> Res<Local> {
        self.unsigned_long()
    }

    fn val(&mut self) -> Res<Val> {
        let place = self.unsigned_int()?;
        let target = self.file.sections.vals as usize + place as usize;
        if let Some((v, levels)) = self.file.vals.get(&target) {
            let (v, levels) = (v.clone(), *levels);
            self.shared(levels)?;
            return Ok(v);
        }
        if self.open_vals.insert(target, ()).is_some() {
            return Err("a value that holds itself".to_string());
        }
        let back = std::mem::replace(&mut self.pos, target);
        let (val, levels) = self.nest(|r| {
            Ok(match r.byte()? {
                1 => Val::True,
                2 => Val::False,
                3 => Val::Null,
                4 => Val::Zero(r.ty()?),
                5 => Val::Char(r.unsigned_int()? as u16),
                6 => Val::Byte(r.byte()? as i8),
                7 => Val::Short(r.signed_int()? as i16),
                8 => Val::Int(r.signed_int()?),
                9 => Val::Long(r.signed_long()?),
                10 => {
                    let bits = u32::from_be_bytes(r.array()?);
                    Val::Float(bits, f32::from_bits(bits).is_nan().then_some(place))
                }
                11 => {
                    let bits = u64::from_be_bytes(r.array()?);
                    Val::Double(bits, f64::from_bits(bits).is_nan().then_some(place))
                }
                12 => Val::StructValue(r.vals()?.into()),
                13 => Val::ArrayValue(r.ty()?, r.vals()?.into()),
                14 => {
                    let n = r.count()?;
                    Val::ByteString(r.take(n)?.into(), place)
                }
                15 => Val::Local(r.local()?, r.ty()?),
                16 => Val::Global(r.global()?, r.ty()?),
                17 => Val::Unit,
                18 => Val::Const(Arc::new(r.val()?)),
                19 => Val::String(r.string()?),
                20 => Val::Virtual(r.unsigned_long()? as i64),
                21 => Val::ClassOf(r.global()?),
                23 => Val::Size(r.unsigned_long()? as i64),
                24 => Val::Int128 { hi: r.signed_long()?, lo: r.unsigned_long()? as i64 },
                tag => return Err(format!("tag {} of a value", tag)),
            })
        })?;
        self.pos = back;
        self.open_vals.remove(&target);
        self.file.vals.insert(target, (val.clone(), levels));
        Ok(val)
    }

    fn vals(&mut self) -> Res<Box<[Val]>> {
        self.seq(Self::val)
    }

    fn bin(&mut self) -> Res<Bin> {
        use Bin::*;
        const ALL: [Bin; 18] = [Iadd, Fadd, Isub, Fsub, Imul, Fmul, Sdiv, Udiv, Fdiv, Srem, Urem, Frem, Shl, Lshr, Ashr, And, Or, Xor];
        let tag = self.byte()?;
        ALL.get((tag as usize).wrapping_sub(1)).copied().ok_or_else(|| format!("tag {} of a binary operation", tag))
    }

    fn comp(&mut self) -> Res<Comp> {
        use Comp::*;
        const ALL: [Comp; 16] = [Ieq, Ine, Ugt, Uge, Ult, Ule, Sgt, Sge, Slt, Sle, Feq, Fne, Fgt, Fge, Flt, Fle];
        let tag = self.byte()?;
        ALL.get((tag as usize).wrapping_sub(1)).copied().ok_or_else(|| format!("tag {} of a comparison", tag))
    }

    fn conv(&mut self) -> Res<Conv> {
        use Conv::*;
        const ALL: [Conv; 14] =
            [Trunc, Zext, Sext, Fptrunc, Fpext, Fptoui, Fptosi, Uitofp, Sitofp, Ptrtoint, Inttoptr, Bitcast, SSizeCast, ZSizeCast];
        let tag = self.byte()?;
        ALL.get((tag as usize).wrapping_sub(1)).copied().ok_or_else(|| format!("tag {} of a conversion", tag))
    }

    fn memory_order(&mut self) -> Res<MemoryOrder> {
        use MemoryOrder::*;
        const ALL: [MemoryOrder; 6] = [Unordered, Monotonic, Acquire, Release, AcqRel, SeqCst];
        let tag = self.byte()?;
        ALL.get((tag as usize).wrapping_sub(1)).copied().ok_or_else(|| format!("tag {} of a memory order", tag))
    }

    fn next(&mut self) -> Res<Next> {
        self.enter()?;
        let next = match self.byte()? {
            1 => Next::None,
            2 => Next::Unwind { exc: self.local()?, ty: self.ty()?, next: Box::new(self.next()?) },
            3 => Next::Case(self.val()?, Box::new(self.next()?)),
            4 => Next::Label(self.local()?, self.vals()?),
            tag => return Err(format!("tag {} of a successor", tag)),
        };
        self.depth -= 1;
        Ok(next)
    }

    fn op(&mut self) -> Res<Op> {
        Ok(match self.byte()? {
            1 => Op::Call { ty: self.ty()?, ptr: self.val()?, args: self.vals()? },
            2 => Op::Load { ty: self.ty()?, ptr: self.val()?, order: None },
            3 => Op::Load { ty: self.ty()?, ptr: self.val()?, order: Some(self.memory_order()?) },
            4 => Op::Store { ty: self.ty()?, ptr: self.val()?, value: self.val()?, order: None },
            5 => Op::Store { ty: self.ty()?, ptr: self.val()?, value: self.val()?, order: Some(self.memory_order()?) },
            6 => Op::Elem { ty: self.ty()?, ptr: self.val()?, indexes: self.vals()? },
            7 => Op::Extract { aggr: self.val()?, indexes: self.seq(Self::signed_int)? },
            8 => Op::Insert { aggr: self.val()?, value: self.val()?, indexes: self.seq(Self::signed_int)? },
            9 => Op::Stackalloc { ty: self.ty()?, n: self.val()? },
            10 => Op::Bin { bin: self.bin()?, ty: self.ty()?, l: self.val()?, r: self.val()? },
            11 => Op::Comp { comp: self.comp()?, ty: self.ty()?, l: self.val()?, r: self.val()? },
            12 => Op::Conv { conv: self.conv()?, ty: self.ty()?, value: self.val()? },
            13 => Op::Classalloc { name: self.global()?, zone: None },
            14 => Op::Classalloc { name: self.global()?, zone: Some(self.val()?) },
            15 => Op::Field { obj: self.val()?, name: self.global()? },
            16 => Op::Fieldload { ty: self.ty()?, obj: self.val()?, name: self.global()? },
            17 => Op::Fieldstore { ty: self.ty()?, obj: self.val()?, name: self.global()?, value: self.val()? },
            18 => Op::Method { obj: self.val()?, sig: self.string()? },
            19 => Op::Module(self.global()?),
            20 => Op::As { ty: self.ty()?, obj: self.val()? },
            21 => Op::Is { ty: self.ty()?, obj: self.val()? },
            22 => Op::Copy(self.val()?),
            23 => Op::SizeOf(self.ty()?),
            24 => Op::AlignmentOf(self.ty()?),
            25 => Op::Box { ty: self.ty()?, obj: self.val()? },
            26 => Op::Unbox { ty: self.ty()?, obj: self.val()? },
            27 => Op::Dynmethod { obj: self.val()?, sig: self.string()? },
            28 => Op::Var(self.ty()?),
            29 => Op::Varload(self.val()?),
            30 => Op::Varstore { slot: self.val()?, value: self.val()? },
            31 => Op::Arrayalloc { ty: self.ty()?, init: self.val()?, zone: None },
            32 => Op::Arrayalloc { ty: self.ty()?, init: self.val()?, zone: Some(self.val()?) },
            33 => Op::Arrayload { ty: self.ty()?, arr: self.val()?, idx: self.val()? },
            34 => Op::Arraystore { ty: self.ty()?, arr: self.val()?, idx: self.val()?, value: self.val()? },
            35 => Op::Arraylength(self.val()?),
            36 => Op::Fence(self.memory_order()?),
            tag => return Err(format!("tag {} of an operation", tag)),
        })
    }

    fn linktime_condition(&mut self) -> Res<LinktimeCondition> {
        self.enter()?;
        let cond = match self.byte()? {
            1 => LinktimeCondition::Simple { property: self.string()?, comparison: self.comp()?, value: self.val()?, pos: self.position()? },
            2 => LinktimeCondition::Complex {
                op: self.bin()?,
                left: Box::new(self.linktime_condition()?),
                right: Box::new(self.linktime_condition()?),
                pos: self.position()?,
            },
            tag => return Err(format!("tag {} of a link-time condition", tag)),
        };
        self.depth -= 1;
        Ok(cond)
    }

    fn inst(&mut self) -> Res<Inst> {
        let tag = self.byte()?;
        let pos = self.position()?;
        let kind = match tag {
            1 => InstKind::Label { id: self.local()?, params: self.seq(|r| Ok((r.local()?, r.ty()?)))? },
            // The scope comes last, after the operation and its handler.
            2 => InstKind::Let { id: self.local()?, op: self.op()?, unwind: Next::None, scope: self.unsigned_int()? },
            3 => InstKind::Let { id: self.local()?, op: self.op()?, unwind: self.next()?, scope: self.unsigned_int()? },
            4 => InstKind::Ret(self.val()?),
            5 => InstKind::Jump(self.next()?),
            6 => InstKind::If { value: self.val()?, then: self.next()?, otherwise: self.next()? },
            7 => InstKind::Switch { value: self.val()?, default: self.next()?, cases: self.seq(Self::next)? },
            8 => InstKind::Throw { value: self.val()?, unwind: self.next()? },
            9 => InstKind::Unreachable { unwind: self.next()? },
            10 => InstKind::LinktimeIf { cond: self.linktime_condition()?, then: self.next()?, otherwise: self.next()? },
            tag => return Err(format!("tag {} of an instruction", tag)),
        };
        Ok(Inst { pos, kind })
    }

    fn attrs(&mut self) -> Res<Attrs> {
        let mut a = Attrs::default();
        for _ in 0..self.count()? {
            match self.byte()? {
                1 => a.inline = Inline::May,
                2 => a.inline = Inline::Hint,
                3 => a.inline = Inline::No,
                4 => a.inline = Inline::Always,
                5 => a.no_specialize = false,
                6 => a.no_specialize = true,
                7 => a.opt = Opt::Un,
                8 => a.opt = Opt::No,
                9 => a.opt = Opt::Did,
                10 => a.opt = Opt::Bail(self.string()?),
                11 => {
                    a.is_extern = true;
                    a.is_blocking = self.bool()?;
                }
                12 => {
                    let link = self.string()?;
                    a.links.push(link);
                }
                13 => a.is_dyn = true,
                14 => a.is_stub = true,
                15 => a.is_abstract = true,
                16 => a.is_volatile = true,
                17 => a.is_final = true,
                18 => a.is_safe_publish = true,
                19 => a.is_linktime_resolved = true,
                20 => a.uses_intrinsic = true,
                21 => a.align = Some((self.signed_int()?, self.opt(Self::string)?)),
                22 => {
                    let define = self.string()?;
                    a.defines.push(define);
                }
                23 => a.link_cpp_runtime = true,
                tag => return Err(format!("tag {} of an attribute", tag)),
            }
        }
        Ok(a)
    }

    fn debug_info(&mut self) -> Res<DebugInfo> {
        let n = self.count()?;
        let mut local_names = Vec::with_capacity(n);
        for _ in 0..n {
            local_names.push((self.local()?, self.string()?));
        }
        let scopes = self.seq(|r| Ok(LexicalScope { id: r.unsigned_int()?, parent: r.unsigned_int()?, pos: r.position()? }))?;
        Ok(DebugInfo::new(local_names, scopes.into_vec()))
    }

    fn defn(&mut self) -> Res<Defn> {
        let tag = self.byte()?;
        let name = self.global()?;
        let attrs = self.attrs()?;
        let pos = self.position()?;
        let kind = match tag {
            1 => DefnKind::Var { ty: self.ty()?, rhs: self.val()? },
            2 => DefnKind::Const { ty: self.ty()?, rhs: self.val()? },
            3 => DefnKind::Declare { ty: self.ty()? },
            4 => DefnKind::Define {
                ty: self.ty()?,
                insts: self.at(self.file.sections.insts, |r| r.seq(Self::inst))?.into_vec(),
                debug: self.debug_info()?,
            },
            5 => DefnKind::Trait { traits: self.globals()? },
            6 => DefnKind::Class { parent: self.opt(Self::global)?, traits: self.globals()? },
            7 => DefnKind::Module { parent: self.opt(Self::global)?, traits: self.globals()? },
            tag => return Err(format!("tag {} of a definition", tag)),
        };
        Ok(Defn { attrs, name, pos, kind })
    }
}

/// UTF-16 code units as UTF-8, a surrogate without its pair as the three bytes its value would
/// take.
fn wtf8(units: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(units.len());
    let mut i = 0;
    while i < units.len() {
        let u = units[i] as u32;
        i += 1;
        let code = match units.get(i) {
            Some(&low) if (0xd800..0xdc00).contains(&u) && (0xdc00..0xe000).contains(&low) => {
                i += 1;
                0x10000 + ((u - 0xd800) << 10) + (low as u32 - 0xdc00)
            }
            _ => u,
        };
        match code {
            0..=0x7f => out.push(code as u8),
            0x80..=0x7ff => out.extend_from_slice(&[0xc0 | (code >> 6) as u8, 0x80 | (code & 0x3f) as u8]),
            0x800..=0xffff => out.extend_from_slice(&[0xe0 | (code >> 12) as u8, 0x80 | ((code >> 6) & 0x3f) as u8, 0x80 | (code & 0x3f) as u8]),
            _ => out.extend_from_slice(&[
                0xf0 | (code >> 18) as u8,
                0x80 | ((code >> 12) & 0x3f) as u8,
                0x80 | ((code >> 6) & 0x3f) as u8,
                0x80 | (code & 0x3f) as u8,
            ]),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prelude(sections: [u32; 8]) -> Vec<u8> {
        let mut file = Vec::new();
        for word in [MAGIC, COMPAT, REVISION].into_iter().chain(sections) {
            file.extend_from_slice(&word.to_be_bytes());
        }
        file.push(0);
        file
    }

    #[test]
    fn refuses_a_symbol_that_owns_itself() {
        // The table's first name is a member whose owner is the member again.
        let mut file = prelude([45, 0, 0, 46, 0, 0, 0, 0]);
        file.extend_from_slice(&[0, 3, 0]);
        let mut cx = Cx::new();
        let refused = NirFile::open(&file).and_then(|mut f| f.offsets(&mut cx));
        assert_eq!(refused, Err("a member whose owner is no class".to_string()));
    }

    #[test]
    fn refuses_a_type_and_a_value_that_hold_themselves() {
        // A declaration of the type `var[itself]`: the definition, the symbol that is none, a
        // position, the empty string and the type.
        let mut file = prelude([45, 54, 51, 50, 55, 45, 0, 0]);
        file.extend_from_slice(&[3, 0, 0, 0, 0, 1, 0, 0, 0, 0, 17, 0]);
        let mut cx = Cx::new();
        assert_eq!(NirFile::open(&file).and_then(|mut f| f.defn(&mut cx, 0)), Err("a type that holds itself".to_string()));
        // A variable of the type `int` whose value is `const itself`.
        let mut file = prelude([45, 55, 52, 51, 56, 45, 57, 0]);
        file.extend_from_slice(&[1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 7, 18, 0]);
        assert_eq!(NirFile::open(&file).and_then(|mut f| f.defn(&mut cx, 0)), Err("a value that holds itself".to_string()));
    }

    /// `n` entries of `tag`, each naming the next by an offset of three bytes, the first at `base`
    /// of its section, and `last` after them.
    fn chain(n: usize, tag: u8, base: usize, last: u8) -> Vec<u8> {
        let mut out = Vec::new();
        for k in 1..=n {
            let next = base + 4 * k;
            out.extend_from_slice(&[tag, 0x80 | next as u8, 0x80 | (next >> 7) as u8, (next >> 14) as u8]);
        }
        out.push(last);
        out
    }

    /// A file of one definition, `A.f`, in which a type, a value, a successor or a link-time
    /// condition reaches `levels` of nesting and names no circle.
    fn nested(shape: &str, levels: usize) -> Vec<u8> {
        // The types `() => unit` and `unit`, the value `null`.
        let (mut types, mut vals, mut insts) = (vec![13, 0, 3, 18], vec![3], Vec::new());
        // A method of the first type: its instructions, no local names, no scopes.
        let method = vec![4, 0, 0, 0, 0, 0, 0, 0];
        let defn = match shape {
            // A variable of the type `var[var[...[unit]]]`.
            "type" => {
                types.extend(chain(levels - 1, 17, 4, 18));
                vec![1, 0, 0, 0, 4, 0]
            }
            // A variable of the value `const const ... null`.
            "value" => {
                vals = chain(levels - 1, 18, 0, 3);
                vec![1, 0, 0, 0, 3, 0]
            }
            // `jump unwind %0 : unit to unwind %0 : unit to ...` nothing.
            "successor" => {
                insts = [vec![1, 5, 0], [2, 0, 3].repeat(levels - 1), vec![1]].concat();
                method
            }
            // A link-time `if` on conditions joined down the left, each leaf `p ieq null`: a leaf's
            // value is one level more.
            _ => {
                let (leaf, joined) = ([1, 10, 1, 0, 0], levels - 2);
                insts = [vec![1, 10, 0], [2, 14].repeat(joined), leaf.to_vec(), [&leaf[..], &[0]].concat().repeat(joined), vec![1, 1]].concat();
                method
            }
        };
        assemble(&types, &defn, &vals, &insts)
    }

    /// A file of the one definition `A.f` with these sections: the table (the definition, then the
    /// symbol that is none), the strings `A`, the empty one, `D1fuEO` and `p`, a position in no
    /// file, the symbols `A.f`, `A` and none, and the types, definitions, values and instructions.
    fn assemble(types: &[u8], defn: &[u8], vals: &[u8], insts: &[u8]) -> Vec<u8> {
        let (table, strings, positions, globals) = ([0, 0, 5, 0], *b"\x01A\x00\x06D1fuEO\x01p", [2, 0, 0], [3, 3, 3, 2, 0, 1]);
        let sections: [&[u8]; 8] = [&table, &strings, &positions, &globals, types, defn, vals, insts];
        let mut end = PRELUDE_LEN as u32;
        let mut file = prelude(sections.map(|section| {
            end += section.len() as u32;
            end - section.len() as u32
        }));
        sections.iter().for_each(|section| file.extend_from_slice(section));
        file
    }

    fn leb(mut n: usize) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let low = (n & 0x7f) as u8;
            n >>= 7;
            if n == 0 {
                out.push(low);
                return out;
            }
            out.push(low | 0x80);
        }
    }

    /// `A.f`, a variable of a struct value (or of a struct type) made of two references to the
    /// one before it, `levels` of them over `null` (over `unit`): two to the power of `levels`
    /// leaves in a file of a few bytes each level.
    fn dag(levels: usize, of_types: bool) -> Vec<u8> {
        let (mut types, mut vals) = (vec![13, 0, 3, 18], vec![3]);
        let section = if of_types { &mut types } else { &mut vals };
        let mut below = if of_types { 3 } else { 0 };
        for _ in 0..levels {
            let at = section.len();
            section.extend([&[12, 2][..], &leb(below), &leb(below)].concat());
            below = at;
        }
        let defn = if of_types { [&[1, 0, 0, 0][..], &leb(below), &[0]].concat() } else { [&[1, 0, 0, 0, 3][..], &leb(below)].concat() };
        assemble(&types, &defn, &vals, &[])
    }

    /// `A.f`, a variable of a struct type (or a struct value) whose members are every `step`-th
    /// entry of one chain of `levels` `var[...]` types over `unit` (of `const ...` over `null`): each
    /// member decoded within the bound, resting on the member before it, decoded already.
    fn split(levels: usize, step: usize, of_types: bool) -> Vec<u8> {
        let (mut types, mut vals) = (vec![13, 0, 3, 18], vec![3]);
        let section = if of_types { &mut types } else { &mut vals };
        let (mut below, mut members) = (if of_types { 3 } else { 0 }, Vec::new());
        for k in 1..=levels {
            let at = section.len();
            let entry = [&[if of_types { 17 } else { 18 }][..], &leb(below)].concat();
            section.extend(&entry);
            section.resize(at + 5, 0);
            below = at;
            if k % step == 0 || k == levels {
                members.push(at);
            }
        }
        let whole = section.len();
        section.extend([&[12][..], &leb(members.len())].concat());
        members.iter().for_each(|&m| section.extend(leb(m)));
        let defn = if of_types { [&[1, 0, 0, 0][..], &leb(whole), &[0]].concat() } else { [&[1, 0, 0, 0, 3][..], &leb(whole)].concat() };
        assemble(&types, &defn, &vals, &[])
    }

    /// The bound's levels decode, one more is refused.
    /// The bound read on the stack the reader is given: the compiler thread's (`src/main.rs`), whose
    /// size a test thread does not have where frames are unoptimised.
    fn holds_the_bound(shape: &'static str) {
        let read = move || {
            let mut cx = Cx::new();
            let within = nested(shape, MAX_DEPTH as usize);
            assert!(NirFile::open(&within).and_then(|mut f| f.defns(&mut cx)).is_ok());
            let beyond = nested(shape, MAX_DEPTH as usize + 1);
            let refused = NirFile::open(&beyond).and_then(|mut f| f.defns(&mut cx)).err();
            assert_eq!(refused, Some(format!("M1AD1fuEO: nested deeper than {}", MAX_DEPTH)));
        };
        let reader = std::thread::Builder::new().stack_size(1 << 30).spawn(read).expect("a thread for the reader");
        if let Err(failure) = reader.join() {
            std::panic::resume_unwind(failure);
        }
    }

    #[test]
    fn refuses_types_nested_deeper_than_the_bound() {
        holds_the_bound("type");
    }

    #[test]
    fn refuses_values_nested_deeper_than_the_bound() {
        holds_the_bound("value");
    }

    #[test]
    fn refuses_successors_nested_deeper_than_the_bound() {
        holds_the_bound("successor");
    }

    #[test]
    fn refuses_conditions_nested_deeper_than_the_bound() {
        holds_the_bound("condition");
    }

    /// A value named twice is decoded once and held once: its references share it, as many levels
    /// down as the file has, where a copy each would double the value at every level.
    #[test]
    fn shares_what_a_file_names_twice() {
        let mut cx = Cx::new();
        let file = dag(40, false);
        let defns = NirFile::open(&file).and_then(|mut f| f.defns(&mut cx)).unwrap();
        let DefnKind::Var { rhs, .. } = &defns[0].kind else { panic!("no variable read") };
        let (mut level, mut levels) = (rhs, 0);
        while let Val::StructValue(parts) = level {
            if let (Val::StructValue(left), Val::StructValue(right)) = (&parts[0], &parts[1]) {
                assert!(Arc::ptr_eq(left, right));
            } else {
                assert!(matches!(&parts[..], [Val::Null, Val::Null]));
            }
            levels += 1;
            level = &parts[0];
        }
        assert_eq!(levels, 40);
        assert!(matches!(level, Val::Null));
        let file = dag(40, true);
        assert!(NirFile::open(&file).and_then(|mut f| f.defns(&mut cx)).is_ok());
    }

    /// A type or a value decoded before counts its levels where it is named again: a chain
    /// decoded in steps holds the bound as one decoded whole does.
    fn holds_the_bound_through_what_was_decoded(of_types: bool) {
        let read = move || {
            let mut cx = Cx::new();
            // The struct is a level, the chain's base another, and each entry of the chain one more.
            let within = split(MAX_DEPTH as usize - 2, 100, of_types);
            assert!(NirFile::open(&within).and_then(|mut f| f.defns(&mut cx)).is_ok());
            let beyond = split(MAX_DEPTH as usize - 1, 100, of_types);
            let refused = NirFile::open(&beyond).and_then(|mut f| f.defns(&mut cx)).err();
            assert_eq!(refused, Some(format!("M1AD1fuEO: nested deeper than {}", MAX_DEPTH)));
        };
        let reader = std::thread::Builder::new().stack_size(1 << 30).spawn(read).expect("a thread for the reader");
        if let Err(failure) = reader.join() {
            std::panic::resume_unwind(failure);
        }
    }

    #[test]
    fn refuses_types_nested_deeper_than_the_bound_in_steps() {
        holds_the_bound_through_what_was_decoded(true);
    }

    #[test]
    fn refuses_values_nested_deeper_than_the_bound_in_steps() {
        holds_the_bound_through_what_was_decoded(false);
    }

    /// A variable of the type `unit` whose value is null, at a position whose offset is 2^31, then
    /// 2^31 - 1, which is read and is beyond this file.
    #[test]
    fn refuses_a_position_beyond_31_bits() {
        let mut cx = Cx::new();
        let at = |offset: [u8; 5]| assemble(&[13, 0, 3, 18], &[&[1, 0, 0][..], &offset, &[3, 0]].concat(), &[3], &[]);
        let read = |file: Vec<u8>, cx: &mut Cx| NirFile::open(&file).and_then(|mut f| f.defns(cx)).err();
        assert_eq!(read(at([0x80, 0x80, 0x80, 0x80, 0x08]), &mut cx), Some("M1AD1fuEO: a position beyond the 31 bits of an offset".to_string()));
        assert_eq!(read(at([0xff, 0xff, 0xff, 0xff, 0x07]), &mut cx), Some("M1AD1fuEO: the file ends within a definition".to_string()));
    }

    #[test]
    fn refuses_another_version_and_a_short_file() {
        let mut newer = prelude([45; 8]);
        newer[11] = 12;
        assert_eq!(NirFile::open(&newer).err(), Some("NIR 6.12, where 6.0 to 6.11 are read".to_string()));
        assert!(NirFile::open(&newer[..PRELUDE_LEN - 1]).is_err());
        assert!(NirFile::open(b"PK\x03\x04 and forty-one bytes more, no NIR in sight").is_err());
    }

    #[test]
    fn reads_an_empty_file() {
        // No definitions: the table is its terminator, the symbol that is none.
        let mut file = prelude([45, 0, 0, 47, 0, 0, 0, 0]);
        file.extend_from_slice(&[0, 0, 1]);
        let mut cx = Cx::new();
        assert_eq!(NirFile::open(&file).and_then(|mut f| f.defns(&mut cx)), Ok(Vec::new()));
    }
}
