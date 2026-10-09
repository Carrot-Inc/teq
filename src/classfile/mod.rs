//! Reads Java class files for their signatures: the constant pool, the access flags, the class,
//! its superclass and interfaces, the fields and methods, and the attributes a type checker
//! needs (JVMS chapter 4). Code and everything else is skipped by length. The stripped class
//! files of the JDK's `lib/ct.sym` are ordinary class files without `Code`, so they read the
//! same way (`jdk.rs`).

pub mod cli;
pub mod jdk;
pub mod shapes;
pub mod show;
pub mod sig;
pub mod stats;

pub const ACC_PUBLIC: u16 = 0x0001;
pub const ACC_PRIVATE: u16 = 0x0002;
pub const ACC_PROTECTED: u16 = 0x0004;
pub const ACC_STATIC: u16 = 0x0008;
pub const ACC_FINAL: u16 = 0x0010;
pub const ACC_BRIDGE: u16 = 0x0040;
pub const ACC_VARARGS: u16 = 0x0080;
pub const ACC_INTERFACE: u16 = 0x0200;
pub const ACC_ABSTRACT: u16 = 0x0400;
pub const ACC_SYNTHETIC: u16 = 0x1000;
pub const ACC_ANNOTATION: u16 = 0x2000;
pub const ACC_ENUM: u16 = 0x4000;
pub const ACC_MODULE: u16 = 0x8000;

/// Nesting of annotation values (`@A(@B(@C))`) and of attribute structures the reader follows.
const MAX_DEPTH: u32 = 32;

pub enum Const {
    Utf8(u32, u32),
    Int(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Class(u16),
    Str(u16),
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConstValue {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Str(String),
}

#[derive(Default, Debug)]
pub struct Field {
    pub access: u16,
    pub name: String,
    pub descriptor: String,
    pub signature: Option<String>,
    pub constant: Option<ConstValue>,
    pub deprecated: bool,
    /// Runtime-visible annotations by the binary name of their class.
    pub annotations: Vec<String>,
}

#[derive(Default, Debug)]
pub struct Method {
    pub access: u16,
    pub name: String,
    pub descriptor: String,
    pub signature: Option<String>,
    pub exceptions: Vec<String>,
    /// From `MethodParameters` when the compiler wrote it; `None` for a parameter without a name.
    pub param_names: Option<Vec<Option<String>>>,
    pub deprecated: bool,
    pub annotations: Vec<String>,
}

#[derive(Debug)]
pub struct InnerClass {
    pub inner: String,
    pub outer: Option<String>,
    /// The simple name; `None` for an anonymous class.
    pub name: Option<String>,
    pub access: u16,
}

#[derive(Debug)]
pub struct RecordComponent {
    pub name: String,
    pub descriptor: String,
    pub signature: Option<String>,
}

#[derive(Default, Debug)]
pub struct ClassFile {
    pub major: u16,
    pub minor: u16,
    pub access: u16,
    /// Binary name, `java/util/Map$Entry`.
    pub name: String,
    pub superclass: Option<String>,
    pub interfaces: Vec<String>,
    pub signature: Option<String>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub inner_classes: Vec<InnerClass>,
    pub record: Option<Vec<RecordComponent>>,
    pub permitted: Vec<String>,
    pub deprecated: bool,
    pub annotations: Vec<String>,
    pub source_file: Option<String>,
    /// Whether the file was written by a Scala compiler (`TASTY`, `Scala` or `ScalaSig` attribute).
    pub scala: bool,
    /// The raw `bytes` of a `ScalaSignature` or `ScalaLongSignature` annotation: the symbol
    /// table scalac 2 pickled, still encoded (`scala2::pickle::decode_signature`).
    pub scala_sig: Option<Vec<u8>>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn need(&self, n: usize) -> Result<(), String> {
        if self.pos + n > self.bytes.len() {
            return Err(format!("truncated class file: {} bytes wanted at offset {}, {} left", n, self.pos, self.bytes.len() - self.pos));
        }
        Ok(())
    }

    fn u8(&mut self) -> Result<u8, String> {
        self.need(1)?;
        let v = self.bytes[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn u16(&mut self) -> Result<u16, String> {
        self.need(2)?;
        let v = u16::from_be_bytes([self.bytes[self.pos], self.bytes[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    fn u32(&mut self) -> Result<u32, String> {
        self.need(4)?;
        let b = &self.bytes[self.pos..self.pos + 4];
        self.pos += 4;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> Result<u64, String> {
        Ok(((self.u32()? as u64) << 32) | self.u32()? as u64)
    }

    fn skip(&mut self, n: usize) -> Result<(), String> {
        self.need(n)?;
        self.pos += n;
        Ok(())
    }
}

struct Pool<'a> {
    bytes: &'a [u8],
    consts: Vec<Const>,
}

impl<'a> Pool<'a> {
    fn get(&self, i: u16) -> Result<&Const, String> {
        self.consts.get(i as usize).filter(|_| i != 0).ok_or_else(|| format!("constant pool index {} out of range", i))
    }

    fn utf8(&self, i: u16) -> Result<String, String> {
        match self.get(i)? {
            Const::Utf8(start, len) => Ok(modified_utf8(&self.bytes[*start as usize..(*start + *len) as usize])),
            _ => Err(format!("constant pool entry {} is not a Utf8", i)),
        }
    }

    fn class(&self, i: u16) -> Result<String, String> {
        match self.get(i)? {
            Const::Class(n) => self.utf8(*n),
            _ => Err(format!("constant pool entry {} is not a Class", i)),
        }
    }

    fn value(&self, i: u16) -> Result<ConstValue, String> {
        Ok(match self.get(i)? {
            Const::Int(v) => ConstValue::Int(*v),
            Const::Long(v) => ConstValue::Long(*v),
            Const::Float(v) => ConstValue::Float(*v),
            Const::Double(v) => ConstValue::Double(*v),
            Const::Str(n) => ConstValue::Str(self.utf8(*n)?),
            _ => return Err(format!("constant pool entry {} is not a value", i)),
        })
    }
}

/// The JVM's modified UTF-8: `\0` as two bytes and supplementary characters as surrogate pairs.
/// ASCII, which every descriptor and nearly every name is, is copied as it stands.
fn modified_utf8(bytes: &[u8]) -> String {
    if bytes.is_ascii() {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    let mut units: Vec<u16> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i] as u16;
        let (unit, len) = if b < 0x80 {
            (b, 1)
        } else if b & 0xe0 == 0xc0 && i + 1 < bytes.len() {
            (((b & 0x1f) << 6) | (bytes[i + 1] as u16 & 0x3f), 2)
        } else if b & 0xf0 == 0xe0 && i + 2 < bytes.len() {
            (((b & 0x0f) << 12) | ((bytes[i + 1] as u16 & 0x3f) << 6) | (bytes[i + 2] as u16 & 0x3f), 3)
        } else {
            (0xfffd, 1)
        };
        units.push(unit);
        i += len;
    }
    String::from_utf16_lossy(&units)
}

impl ClassFile {
    pub fn parse(bytes: &[u8]) -> Result<ClassFile, String> {
        let mut r = Reader { bytes, pos: 0 };
        if r.u32()? != 0xcafe_babe {
            return Err("not a class file (bad magic number)".to_string());
        }
        let minor = r.u16()?;
        let major = r.u16()?;
        let pool = read_pool(&mut r)?;
        let mut cf = ClassFile { major, minor, ..ClassFile::default() };
        cf.access = r.u16()?;
        cf.name = pool.class(r.u16()?)?;
        let sup = r.u16()?;
        cf.superclass = if sup == 0 { None } else { Some(pool.class(sup)?) };
        for _ in 0..r.u16()? {
            cf.interfaces.push(pool.class(r.u16()?)?);
        }
        for _ in 0..r.u16()? {
            let mut f = Field { access: r.u16()?, name: pool.utf8(r.u16()?)?, descriptor: pool.utf8(r.u16()?)?, ..Field::default() };
            for _ in 0..r.u16()? {
                let (name, len) = attribute_head(&mut r, &pool)?;
                let end = r.pos + len;
                match name.as_str() {
                    "Signature" => f.signature = Some(pool.utf8(r.u16()?)?),
                    "ConstantValue" => f.constant = Some(pool.value(r.u16()?)?),
                    "Deprecated" => f.deprecated = true,
                    "RuntimeVisibleAnnotations" => f.annotations = annotations(&mut r, &pool)?,
                    _ => {}
                }
                r.pos = end;
            }
            cf.fields.push(f);
        }
        for _ in 0..r.u16()? {
            let mut m = Method { access: r.u16()?, name: pool.utf8(r.u16()?)?, descriptor: pool.utf8(r.u16()?)?, ..Method::default() };
            for _ in 0..r.u16()? {
                let (name, len) = attribute_head(&mut r, &pool)?;
                let end = r.pos + len;
                match name.as_str() {
                    "Signature" => m.signature = Some(pool.utf8(r.u16()?)?),
                    "Exceptions" => {
                        for _ in 0..r.u16()? {
                            m.exceptions.push(pool.class(r.u16()?)?);
                        }
                    }
                    "Deprecated" => m.deprecated = true,
                    "RuntimeVisibleAnnotations" => m.annotations = annotations(&mut r, &pool)?,
                    "MethodParameters" => {
                        let mut names = Vec::new();
                        for _ in 0..r.u8()? {
                            let n = r.u16()?;
                            let _flags = r.u16()?;
                            names.push(if n == 0 { None } else { Some(pool.utf8(n)?) });
                        }
                        m.param_names = Some(names);
                    }
                    _ => {}
                }
                r.pos = end;
            }
            cf.methods.push(m);
        }
        for _ in 0..r.u16()? {
            let (name, len) = attribute_head(&mut r, &pool)?;
            let end = r.pos + len;
            match name.as_str() {
                "Signature" => cf.signature = Some(pool.utf8(r.u16()?)?),
                "SourceFile" => cf.source_file = Some(pool.utf8(r.u16()?)?),
                "Deprecated" => cf.deprecated = true,
                "RuntimeVisibleAnnotations" => {
                    let mut sig = Vec::new();
                    cf.annotations = annotations_with_signature(&mut r, &pool, &mut sig)?;
                    if cf.annotations.iter().any(|a| a == "scala/reflect/ScalaSignature" || a == "scala/reflect/ScalaLongSignature") {
                        cf.scala_sig = Some(sig);
                    }
                }
                "InnerClasses" => {
                    for _ in 0..r.u16()? {
                        let inner = pool.class(r.u16()?)?;
                        let outer = r.u16()?;
                        let name = r.u16()?;
                        let access = r.u16()?;
                        cf.inner_classes.push(InnerClass {
                            inner,
                            outer: if outer == 0 { None } else { Some(pool.class(outer)?) },
                            name: if name == 0 { None } else { Some(pool.utf8(name)?) },
                            access,
                        });
                    }
                }
                "Record" => {
                    let mut components = Vec::new();
                    for _ in 0..r.u16()? {
                        let mut c = RecordComponent { name: pool.utf8(r.u16()?)?, descriptor: pool.utf8(r.u16()?)?, signature: None };
                        for _ in 0..r.u16()? {
                            let (name, len) = attribute_head(&mut r, &pool)?;
                            let end = r.pos + len;
                            if name == "Signature" {
                                c.signature = Some(pool.utf8(r.u16()?)?);
                            }
                            r.pos = end;
                        }
                        components.push(c);
                    }
                    cf.record = Some(components);
                }
                "PermittedSubclasses" => {
                    for _ in 0..r.u16()? {
                        cf.permitted.push(pool.class(r.u16()?)?);
                    }
                }
                "TASTY" | "Scala" | "ScalaSig" => cf.scala = true,
                _ => {}
            }
            r.pos = end;
        }
        Ok(cf)
    }

    pub fn is_interface(&self) -> bool {
        self.access & ACC_INTERFACE != 0
    }

    /// The entry of `InnerClasses` that describes this class, when it is nested.
    pub fn own_inner_entry(&self) -> Option<&InnerClass> {
        self.inner_classes.iter().find(|i| i.inner == self.name)
    }

    /// The outer class of a named inner class, one whose instances hold an outer instance.
    pub fn inner_outer(&self) -> Option<&str> {
        let own = self.own_inner_entry()?;
        if own.access & ACC_STATIC != 0 || own.name.is_none() {
            return None;
        }
        own.outer.as_deref()
    }

    /// The nested classes declared in this class, in the order of the attribute.
    pub fn member_classes(&self) -> impl Iterator<Item = &InnerClass> {
        self.inner_classes.iter().filter(move |i| i.outer.as_deref() == Some(self.name.as_str()))
    }
}

fn read_pool<'a>(r: &mut Reader<'a>) -> Result<Pool<'a>, String> {
    let count = r.u16()? as usize;
    let mut consts = Vec::with_capacity(count);
    consts.push(Const::Other);
    while consts.len() < count {
        let tag = r.u8()?;
        let c = match tag {
            1 => {
                let len = r.u16()? as usize;
                let start = r.pos;
                r.skip(len)?;
                Const::Utf8(start as u32, len as u32)
            }
            3 => Const::Int(r.u32()? as i32),
            4 => Const::Float(f32::from_bits(r.u32()?)),
            5 => Const::Long(r.u64()? as i64),
            6 => Const::Double(f64::from_bits(r.u64()?)),
            7 => Const::Class(r.u16()?),
            8 => Const::Str(r.u16()?),
            9 | 10 | 11 | 12 | 17 | 18 => {
                r.skip(4)?;
                Const::Other
            }
            15 => {
                r.skip(3)?;
                Const::Other
            }
            16 | 19 | 20 => {
                r.skip(2)?;
                Const::Other
            }
            t => return Err(format!("unknown constant pool tag {} at offset {}", t, r.pos - 1)),
        };
        let wide = matches!(c, Const::Long(_) | Const::Double(_));
        consts.push(c);
        if wide {
            if consts.len() == count {
                return Err("a long or double constant takes the last pool slot".to_string());
            }
            consts.push(Const::Other);
        }
    }
    Ok(Pool { bytes: r.bytes, consts })
}

fn attribute_head(r: &mut Reader, pool: &Pool) -> Result<(String, usize), String> {
    let name = pool.utf8(r.u16()?)?;
    let len = r.u32()? as usize;
    r.need(len)?;
    Ok((name, len))
}

/// The classes of a `RuntimeVisibleAnnotations` attribute; the element values are walked to
/// find where each annotation ends.
fn annotations(r: &mut Reader, pool: &Pool) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for _ in 0..r.u16()? {
        out.push(annotation(r, pool, 0)?);
    }
    Ok(out)
}

/// The classes of a class's annotations, with the raw bytes of the strings scalac 2's
/// signature annotation holds appended to `sig`.
fn annotations_with_signature(r: &mut Reader, pool: &Pool, sig: &mut Vec<u8>) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for _ in 0..r.u16()? {
        out.push(annotation_into(r, pool, 0, Some(&mut *sig))?);
    }
    Ok(out)
}

fn annotation(r: &mut Reader, pool: &Pool, depth: u32) -> Result<String, String> {
    annotation_into(r, pool, depth, None)
}

fn annotation_into(r: &mut Reader, pool: &Pool, depth: u32, mut sig: Option<&mut Vec<u8>>) -> Result<String, String> {
    let descriptor = pool.utf8(r.u16()?)?;
    let name = descriptor.strip_prefix('L').and_then(|d| d.strip_suffix(';')).unwrap_or(&descriptor).to_string();
    let signature = matches!(name.as_str(), "scala/reflect/ScalaSignature" | "scala/reflect/ScalaLongSignature");
    for _ in 0..r.u16()? {
        let element_name = r.u16()?;
        let sink = if signature && pool.utf8(element_name)? == "bytes" { sig.as_deref_mut() } else { None };
        element_value(r, pool, depth + 1, sink)?;
    }
    Ok(name)
}

fn element_value(r: &mut Reader, pool: &Pool, depth: u32, mut sink: Option<&mut Vec<u8>>) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("annotation values nested too deeply".to_string());
    }
    match r.u8()? {
        b's' if sink.is_some() => {
            if let Const::Utf8(start, len) = pool.get(r.u16()?)? {
                sink.unwrap().extend_from_slice(&pool.bytes[*start as usize..(*start + *len) as usize]);
            }
            Ok(())
        }
        b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's' | b'c' => r.skip(2),
        b'e' => r.skip(4),
        b'@' => annotation(r, pool, depth).map(|_| ()),
        b'[' => {
            for _ in 0..r.u16()? {
                element_value(r, pool, depth + 1, sink.as_deref_mut())?;
            }
            Ok(())
        }
        t => Err(format!("unknown annotation value tag {} at offset {}", t, r.pos - 1)),
    }
}
