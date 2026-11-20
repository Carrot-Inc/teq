//! What a class file says of the work its creation does, for the interpreter, which has no Java body to run: whether
//! a constructor is empty, whether the class has a static initializer, and whether an interface declares the methods
//! that make it initialized with a class implementing it (JVMS 5.5, step 7). Read from the class file's own bytes (a
//! jar's or a directory's entry, the runtime image's resource), never from `ct.sym`'s, which keeps no `Code`, no
//! `<clinit>` and no private method.
//!
//! An empty constructor is exactly `aload_0; invokespecial <its superclass>.<init>()V`, then any number of
//! `aload_0; <the zero of the field's type>; putfield <a field this class declares>`, then `return`, with no
//! exception handler; `java/lang/Object`'s, which has no superclass, is `return` alone. After it every field the
//! class declares holds the default the interpreter gives, provided the superclass's construction did not write
//! them first: what the caller checks of the levels above.
use super::modified_utf8;

const ACC_STATIC: u16 = 0x0008;
const ACC_ABSTRACT: u16 = 0x0400;

const ALOAD_0: u8 = 0x2a;
const INVOKESPECIAL: u8 = 0xb7;
const PUTFIELD: u8 = 0xb5;
const RETURN: u8 = 0xb1;

/// What a class file says of its initialization and constructors.
#[derive(Debug)]
pub struct InitFacts {
    /// Binary name, `java/util/AbstractList`.
    pub name: String,
    pub superclass: Option<String>,
    /// A `<clinit>`, which the JVM runs where it initializes the class.
    pub static_initializer: bool,
    /// A method neither abstract nor static (a default, a private instance method): an interface declaring one is
    /// initialized with every class that implements it.
    pub concrete_instance_method: bool,
    ctors: Vec<(String, Ctor)>,
}

/// One constructor, judged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctor {
    /// The pattern: `stores` where it stores a default into a field of its own.
    Empty { stores: bool },
    /// Anything else, said where it leaves the pattern.
    Other(String),
}

enum Const {
    Utf8(usize, usize),
    Class(u16),
    Ref(u16, u16),
    NameAndType(u16, u16),
    Other,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.bytes.len()).ok_or_else(|| format!("truncated class file at offset {}", self.pos))?;
        let s = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?; // a u4 (the jdk's JVMS 4.1)
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
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
            Const::Utf8(at, len) => Ok(modified_utf8(&self.bytes[*at..*at + *len])),
            _ => Err(format!("constant pool entry {} is not a Utf8", i)),
        }
    }
    fn class(&self, i: u16) -> Result<String, String> {
        match self.get(i)? {
            Const::Class(n) => self.utf8(*n),
            _ => Err(format!("constant pool entry {} is not a Class", i)),
        }
    }
    /// A field's or a method's reference: its class, name and descriptor.
    fn reference(&self, i: u16) -> Result<(String, String, String), String> {
        let Const::Ref(class, nat) = self.get(i)? else { return Err(format!("constant pool entry {} is not a reference", i)) };
        let Const::NameAndType(name, descriptor) = self.get(*nat)? else { return Err(format!("constant pool entry {} is not a NameAndType", nat)) };
        Ok((self.class(*class)?, self.utf8(*name)?, self.utf8(*descriptor)?))
    }
}

impl InitFacts {
    pub fn read(bytes: &[u8]) -> Result<InitFacts, String> {
        let mut r = Reader { bytes, pos: 0 };
        if r.u32()? != 0xCAFE_BABE {
            return Err("not a class file".to_string());
        }
        r.u16()?;
        r.u16()?;
        let count = r.u16()? as usize;
        let mut consts = Vec::with_capacity(count);
        consts.push(Const::Other);
        while consts.len() < count {
            let tag = r.u8()?;
            let c = match tag {
                1 => {
                    let len = r.u16()? as usize;
                    let at = r.pos;
                    r.take(len)?;
                    Const::Utf8(at, len)
                }
                7 => Const::Class(r.u16()?),
                9 | 10 | 11 => Const::Ref(r.u16()?, r.u16()?),
                12 => Const::NameAndType(r.u16()?, r.u16()?),
                3 | 4 => {
                    r.take(4)?; // Integer, Float: the jdk's JVMS 4.4.4
                    Const::Other
                }
                5 | 6 => {
                    r.take(8)?; // Long, Double, two entries: the jdk's JVMS 4.4.5
                    consts.push(Const::Other);
                    Const::Other
                }
                8 | 16 | 19 | 20 => {
                    r.take(2)?;
                    Const::Other
                }
                15 => {
                    r.take(3)?; // MethodHandle: the jdk's JVMS 4.4.8
                    Const::Other
                }
                17 | 18 => {
                    r.take(4)?; // Dynamic, InvokeDynamic: the jdk's JVMS 4.4.10
                    Const::Other
                }
                t => return Err(format!("constant pool tag {}", t)),
            };
            consts.push(c);
        }
        let pool = Pool { bytes, consts };
        // The access flags: the walk takes whether a class is an interface from its typed record too.
        r.u16()?;
        let name = pool.class(r.u16()?)?;
        let superclass = match r.u16()? {
            0 => None,
            i => Some(pool.class(i)?),
        };
        // The interfaces: the walk takes a class's ancestors from its typed record.
        let interfaces = r.u16()? as usize;
        r.take(interfaces * 2)?;
        let mut own_fields: Vec<(String, String)> = Vec::new();
        for _ in 0..r.u16()? {
            let (faccess, fname, fdesc) = (r.u16()?, r.u16()?, r.u16()?);
            if faccess & ACC_STATIC == 0 {
                own_fields.push((pool.utf8(fname)?, pool.utf8(fdesc)?));
            }
            skip_attributes(&mut r)?;
        }
        let mut facts = InitFacts {
            name,
            superclass,
            static_initializer: false,
            concrete_instance_method: false,
            ctors: Vec::new(),
        };
        for _ in 0..r.u16()? {
            let (maccess, mname, mdesc) = (r.u16()?, pool.utf8(r.u16()?)?, pool.utf8(r.u16()?)?);
            let mut code: Option<(&[u8], u16)> = None;
            for _ in 0..r.u16()? {
                let aname = pool.utf8(r.u16()?)?;
                let len = r.u32()? as usize;
                let body = r.take(len)?;
                if aname == "Code" {
                    let mut c = Reader { bytes: body, pos: 0 };
                    c.u16()?;
                    c.u16()?;
                    let n = c.u32()? as usize;
                    let instructions = c.take(n)?;
                    let handlers = c.u16()?;
                    code = Some((instructions, handlers));
                }
            }
            match mname.as_str() {
                "<clinit>" => facts.static_initializer = true,
                "<init>" => {
                    let judged = match code {
                        Some((_, handlers)) if handlers != 0 => Ctor::Other("it has an exception handler".to_string()),
                        Some((code, _)) => judge(&pool, &facts, &own_fields, code)?,
                        None => Ctor::Other("it has no code (abstract or native)".to_string()),
                    };
                    facts.ctors.push((mdesc, judged));
                }
                _ if maccess & (ACC_ABSTRACT | ACC_STATIC) == 0 => facts.concrete_instance_method = true,
                _ => {}
            }
        }
        Ok(facts)
    }

    /// The constructor of descriptor `descriptor` (`()V`, `(I)V`), judged; `None` where the class has none.
    pub fn ctor(&self, descriptor: &str) -> Option<&Ctor> {
        self.ctors.iter().find(|(d, _)| d == descriptor).map(|(_, c)| c)
    }

    /// The descriptors of the class's constructors.
    pub fn ctor_descriptors(&self) -> impl Iterator<Item = &str> {
        self.ctors.iter().map(|(d, _)| d.as_str())
    }
}

fn skip_attributes(r: &mut Reader) -> Result<(), String> {
    for _ in 0..r.u16()? {
        r.u16()?;
        let len = r.u32()? as usize;
        r.take(len)?;
    }
    Ok(())
}

/// The zero a store of `op` writes, for a field of descriptor `descriptor`: `aconst_null` for a reference or an
/// array, `iconst_0` for `int` and the types narrower, `lconst_0`, `fconst_0`, `dconst_0`.
fn zero_of(op: u8, descriptor: &str) -> bool {
    match op {
        0x01 => descriptor.starts_with('L') || descriptor.starts_with('['),
        0x03 => matches!(descriptor, "I" | "Z" | "B" | "C" | "S"),
        0x09 => descriptor == "J",
        0x0b => descriptor == "F",
        0x0e => descriptor == "D",
        _ => false,
    }
}

fn judge(pool: &Pool, facts: &InitFacts, own_fields: &[(String, String)], code: &[u8]) -> Result<Ctor, String> {
    let Some(sup) = facts.superclass.as_deref() else {
        return Ok(if code == [RETURN] { Ctor::Empty { stores: false } } else { Ctor::Other("java/lang/Object's constructor does more than return".to_string()) });
    };
    let [ALOAD_0, INVOKESPECIAL, hi, lo, rest @ ..] = code else {
        return Ok(Ctor::Other("it does not begin by calling a constructor of its superclass".to_string()));
    };
    let (class, name, descriptor) = pool.reference(u16::from_be_bytes([*hi, *lo]))?;
    if class != sup || name != "<init>" || descriptor != "()V" {
        return Ok(Ctor::Other(format!("it begins by calling {}.{}{}, not {}.<init>()V", class, name, descriptor, sup)));
    }
    let mut stores = false;
    let mut at = rest;
    loop {
        match at {
            [RETURN] => return Ok(Ctor::Empty { stores }),
            [ALOAD_0, op, PUTFIELD, hi, lo, more @ ..] => {
                let (class, name, descriptor) = pool.reference(u16::from_be_bytes([*hi, *lo]))?;
                let own = class == facts.name && own_fields.iter().any(|(n, d)| *n == name && *d == descriptor);
                if !own || !zero_of(*op, &descriptor) {
                    return Ok(Ctor::Other(format!("it stores into {}.{} what is not a default of a field of its own", class, name)));
                }
                stores = true;
                at = more;
            }
            [op, ..] => return Ok(Ctor::Other(format!("it runs the instruction 0x{:02x} at offset {}", op, code.len() - at.len()))),
            [] => return Ok(Ctor::Other("it does not end in a return".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ACC_INTERFACE: u16 = 0x0200;

    /// A class file made by hand: the constant pool as the instructions name it, then the class.
    #[derive(Default)]
    struct Writer {
        pool: Vec<Vec<u8>>,
    }

    enum Op<'a> {
        Bytes(&'a [u8]),
        Special(&'a str, &'a str, &'a str),
        Put(&'a str, &'a str, &'a str),
    }

    impl Writer {
        fn add(&mut self, entry: Vec<u8>) -> u16 {
            if let Some(i) = self.pool.iter().position(|e| *e == entry) {
                return i as u16 + 1;
            }
            self.pool.push(entry);
            self.pool.len() as u16
        }
        fn utf8(&mut self, s: &str) -> u16 {
            let mut e = vec![1];
            e.extend_from_slice(&(s.len() as u16).to_be_bytes());
            e.extend_from_slice(s.as_bytes());
            self.add(e)
        }
        fn class(&mut self, name: &str) -> u16 {
            let n = self.utf8(name);
            self.add([&[7u8][..], &n.to_be_bytes()].concat())
        }
        fn reference(&mut self, tag: u8, class: &str, name: &str, descriptor: &str) -> u16 {
            let c = self.class(class);
            let (n, d) = (self.utf8(name), self.utf8(descriptor));
            let nat = self.add([&[12u8][..], &n.to_be_bytes(), &d.to_be_bytes()].concat());
            self.add([&[tag][..], &c.to_be_bytes(), &nat.to_be_bytes()].concat())
        }
        fn code(&mut self, ops: &[Op]) -> Vec<u8> {
            let mut out = Vec::new();
            for op in ops {
                match op {
                    Op::Bytes(b) => out.extend_from_slice(b),
                    Op::Special(c, n, d) => {
                        out.extend_from_slice(&[ALOAD_0, INVOKESPECIAL]);
                        out.extend_from_slice(&self.reference(10, c, n, d).to_be_bytes());
                    }
                    Op::Put(c, n, d) => {
                        out.push(PUTFIELD);
                        out.extend_from_slice(&self.reference(9, c, n, d).to_be_bytes());
                    }
                }
            }
            out
        }
        /// `methods`: access, name, descriptor and code (none for an abstract one).
        fn write(mut self, access: u16, this: &str, sup: Option<&str>, interfaces: &[&str], fields: &[(u16, &str, &str)], methods: Vec<(u16, &str, &str, Option<Vec<u8>>)>) -> Vec<u8> {
            let this_i = self.class(this);
            let sup_i = sup.map_or(0, |s| self.class(s));
            let iface_i: Vec<u16> = interfaces.iter().map(|i| self.class(i)).collect();
            let field_i: Vec<(u16, u16, u16)> = fields.iter().map(|(a, n, d)| (*a, self.utf8(n), self.utf8(d))).collect();
            let code_name = self.utf8("Code");
            let method_i: Vec<(u16, u16, u16, Option<Vec<u8>>)> = methods.into_iter().map(|(a, n, d, c)| (a, self.utf8(n), self.utf8(d), c)).collect();
            let mut out = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 52];
            out.extend_from_slice(&(self.pool.len() as u16 + 1).to_be_bytes());
            for e in &self.pool {
                out.extend_from_slice(e);
            }
            for v in [access, this_i, sup_i, iface_i.len() as u16] {
                out.extend_from_slice(&v.to_be_bytes());
            }
            for i in iface_i {
                out.extend_from_slice(&i.to_be_bytes());
            }
            out.extend_from_slice(&(field_i.len() as u16).to_be_bytes());
            for (a, n, d) in field_i {
                for v in [a, n, d, 0] {
                    out.extend_from_slice(&v.to_be_bytes());
                }
            }
            out.extend_from_slice(&(method_i.len() as u16).to_be_bytes());
            for (a, n, d, code) in method_i {
                for v in [a, n, d, code.is_some() as u16] {
                    out.extend_from_slice(&v.to_be_bytes());
                }
                if let Some(code) = code {
                    out.extend_from_slice(&code_name.to_be_bytes());
                    out.extend_from_slice(&(12 + code.len() as u32).to_be_bytes());
                    out.extend_from_slice(&[0, 2, 0, 1]);
                    out.extend_from_slice(&(code.len() as u32).to_be_bytes());
                    out.extend_from_slice(&code);
                    out.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
            out.extend_from_slice(&[0, 0]);
            out
        }
    }

    const PUBLIC: u16 = 0x0001;

    /// `Bounds`, a subclass of `java/lang/Object` with an `int` and a `String` field, whose constructors are `ctors`.
    fn bounds(ctors: &[(&str, &[Op])]) -> InitFacts {
        let mut w = Writer::default();
        let methods = ctors.iter().map(|(d, ops)| (PUBLIC, "<init>", *d, Some(w.code(ops)))).collect();
        let bytes = w.write(PUBLIC, "jlib/Bounds", Some("java/lang/Object"), &[], &[(PUBLIC, "size", "I"), (PUBLIC, "label", "Ljava/lang/String;"), (PUBLIC | ACC_STATIC, "count", "I")], methods);
        InitFacts::read(&bytes).unwrap()
    }

    const SUPER: Op = Op::Special("java/lang/Object", "<init>", "()V");

    #[test]
    fn the_pattern_by_constructor() {
        let facts = bounds(&[
            ("()V", &[SUPER, Op::Bytes(&[RETURN])]),
            ("(I)V", &[SUPER, Op::Bytes(&[ALOAD_0, 0x1b]), Op::Put("jlib/Bounds", "size", "I"), Op::Bytes(&[RETURN])]),
            ("(J)V", &[SUPER, Op::Bytes(&[ALOAD_0, 0x03]), Op::Put("jlib/Bounds", "size", "I"), Op::Bytes(&[ALOAD_0, 0x01]), Op::Put("jlib/Bounds", "label", "Ljava/lang/String;"), Op::Bytes(&[RETURN])]),
        ]);
        assert_eq!(facts.ctor("()V"), Some(&Ctor::Empty { stores: false }));
        assert!(matches!(facts.ctor("(I)V"), Some(Ctor::Other(_))));
        assert_eq!(facts.ctor("(J)V"), Some(&Ctor::Empty { stores: true }));
        assert_eq!(facts.ctor("(D)V"), None);
        assert!(!facts.static_initializer);
    }

    #[test]
    fn what_leaves_the_pattern() {
        let refused: Vec<(&str, Vec<Op>)> = vec![
            // A store of a constant that is not the field's zero.
            ("one", vec![SUPER, Op::Bytes(&[ALOAD_0, 0x04]), Op::Put("jlib/Bounds", "size", "I"), Op::Bytes(&[RETURN])]),
            // A zero of another type than the field's.
            ("long zero", vec![SUPER, Op::Bytes(&[ALOAD_0, 0x09]), Op::Put("jlib/Bounds", "size", "I"), Op::Bytes(&[RETURN])]),
            // A field the class does not declare, though the reference names the class (javac's form for an inherited one).
            ("inherited", vec![SUPER, Op::Bytes(&[ALOAD_0, 0x03]), Op::Put("jlib/Bounds", "depth", "I"), Op::Bytes(&[RETURN])]),
            // Its static field.
            ("static", vec![SUPER, Op::Bytes(&[ALOAD_0, 0x03]), Op::Put("jlib/Bounds", "count", "I"), Op::Bytes(&[RETURN])]),
            // Another class's field.
            ("other class", vec![SUPER, Op::Bytes(&[ALOAD_0, 0x03]), Op::Put("jlib/Other", "size", "I"), Op::Bytes(&[RETURN])]),
            // `this()`, a delegation to another of its constructors.
            ("delegation", vec![Op::Special("jlib/Bounds", "<init>", "()V"), Op::Bytes(&[RETURN])]),
            // The superclass's constructor of another descriptor.
            ("super(1)", vec![Op::Bytes(&[ALOAD_0, 0x04, INVOKESPECIAL, 0, 0]), Op::Bytes(&[RETURN])]),
            // An instance initializer with an effect: `{ System.out.println("block"); }` begins with a getstatic.
            ("effect", vec![SUPER, Op::Bytes(&[0xb2, 0, 1]), Op::Bytes(&[RETURN])]),
            // Nothing after the super call.
            ("no return", vec![SUPER]),
        ];
        for (what, ops) in refused {
            let facts = bounds(&[("()V", &ops)]);
            assert!(matches!(facts.ctor("()V"), Some(Ctor::Other(_))), "{}: {:?}", what, facts.ctor("()V"));
        }
    }

    #[test]
    fn an_interface_initialized_with_its_classes() {
        let mut w = Writer::default();
        let clinit = w.code(&[Op::Bytes(&[RETURN])]);
        let body = w.code(&[Op::Bytes(&[0x06, 0xac])]);
        let bytes = w.write(
            PUBLIC | ACC_INTERFACE | ACC_ABSTRACT,
            "jlib/Gauge",
            Some("java/lang/Object"),
            &["jlib/Base"],
            &[],
            vec![(ACC_STATIC, "<clinit>", "()V", Some(clinit)), (PUBLIC, "level", "()I", Some(body)), (PUBLIC | ACC_ABSTRACT, "reset", "()V", None)],
        );
        let facts = InitFacts::read(&bytes).unwrap();
        assert!(facts.static_initializer && facts.concrete_instance_method);
        let w = Writer::default();
        let bytes = w.write(PUBLIC | ACC_INTERFACE | ACC_ABSTRACT, "jlib/Plain", Some("java/lang/Object"), &[], &[], vec![(PUBLIC | ACC_ABSTRACT, "reset", "()V", None)]);
        let facts = InitFacts::read(&bytes).unwrap();
        assert!(!facts.static_initializer && !facts.concrete_instance_method);
    }

    #[test]
    fn the_jdks_own_classes() {
        let home = crate::classfile::jdk::find_home().expect("the unit tests read the JDK's runtime image: set JAVA_HOME or put java on the path");
        let mut img = crate::classfile::jimage::Image::open(&home).unwrap();
        let mut facts = |module: &str, class: &str| InitFacts::read(&img.class_bytes(module, class).unwrap()).unwrap();
        let object = facts("java.base", "java/lang/Object");
        assert_eq!(object.ctor("()V"), Some(&Ctor::Empty { stores: false }));
        for (class, stores) in [("java/util/AbstractCollection", false), ("java/util/AbstractList", true), ("java/util/AbstractMap", false)] {
            let f = facts("java.base", class);
            assert_eq!(f.ctor("()V"), Some(&Ctor::Empty { stores }), "{}", class);
            assert!(!f.static_initializer, "{}", class);
        }
        let point = facts("java.desktop", "java/awt/Point");
        assert!(matches!(point.ctor("(II)V"), Some(Ctor::Other(_))));
        let list = facts("java.base", "java/util/ArrayList");
        assert!(matches!(list.ctor("()V"), Some(Ctor::Other(_))) && list.static_initializer);
        let thread = facts("java.base", "java/lang/Thread");
        assert!(matches!(thread.ctor("()V"), Some(Ctor::Other(_))));
        assert!(matches!(facts("java.base", "java/util/HashMap").ctor("()V"), Some(Ctor::Other(_))));
        let checksum = facts("java.base", "java/util/zip/Checksum");
        assert!(checksum.static_initializer && checksum.concrete_instance_method);
    }
}
