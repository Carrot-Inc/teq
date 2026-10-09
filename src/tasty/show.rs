//! Prints decoded signatures in Scala-like syntax, for `teq tasty` and its golden tests.

use super::tags::*;
use super::terms::*;
use super::tree::*;
use super::{NameRef, TName, TastyFile};

/// Whether a body's selections print the signature they select by (`teq tasty --body
/// --signatures`), which tells apart two overloads of one name.
pub static SIGNATURES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub struct Printer<'a> {
    pub file: &'a TastyFile,
    pub decoder: Decoder<'a>,
    /// Whether package prefixes are written out.
    pub full_names: bool,
    /// The lambda types around the type being printed, for `ParamRef`, each with whether it is
    /// a method type, whose parameters are terms.
    binders: Vec<(Addr, Vec<NameRef>, bool)>,
    pub out: String,
}

const KEYWORDS: &[(u8, &str)] = &[
    (PRIVATE, "private"),
    (PROTECTED, "protected"),
    (FINAL, "final"),
    (SEALED, "sealed"),
    (ABSTRACT, "abstract"),
    (OPEN, "open"),
    (OVERRIDE, "override"),
    (IMPLICIT, "implicit"),
    (ERASED, "erased"),
    (LAZY, "lazy"),
    (OPAQUE, "opaque"),
    (TRANSPARENT, "transparent"),
    (INLINE, "inline"),
    (INFIX, "infix"),
    (TRACKED, "tracked"),
    (INTO, "into"),
];

const MARKERS: &[(u8, &str)] = &[
    (SYNTHETIC, "synthetic"),
    (ARTIFACT, "artifact"),
    (EXPORTED, "exported"),
    (MACRO, "macro"),
    (CASEACCESSOR, "caseaccessor"),
    (STATIC, "static"),
    (INVISIBLE, "invisible"),
];

impl<'a> Printer<'a> {
    pub fn new(file: &'a TastyFile, full_names: bool) -> Printer<'a> {
        Printer { file, decoder: Decoder::new(file), full_names, binders: Vec::new(), out: String::new() }
    }

    fn name(&self, n: NameRef) -> String {
        self.file.name(n)
    }

    fn is_scala_type(&self, t: &TType, wanted: &str) -> bool {
        match t {
            TType::TypeRef(prefix, n) => {
                self.file.simple(*n) == Some(wanted)
                    && matches!(&**prefix, TType::Package(p) if self.file.simple(*p) == Some("scala"))
            }
            _ => false,
        }
    }

    /// The name of a type in package `scala`, which function, tuple and repeated types are.
    fn scala_name(&self, t: &TType) -> Option<&'a str> {
        match t {
            TType::TypeRef(prefix, n) => match &**prefix {
                TType::Package(p) if self.file.simple(*p) == Some("scala") => self.file.simple(*n),
                TType::This(inner) if matches!(&**inner, TType::Package(p) if self.file.simple(*p) == Some("scala")) => {
                    self.file.simple(*n)
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn local_name(&self, addr: Addr) -> String {
        match self.decoder.name_at(addr) {
            Some(n) => self.name(n),
            None => format!("<local@{}>", addr),
        }
    }

    /// A prefix as the text in front of a member name, empty for packages and `this`.
    fn prefix(&mut self, t: &TType) -> String {
        match t {
            TType::Package(p) if self.full_names => format!("{}.", self.name(*p)),
            TType::Package(_) => String::new(),
            TType::This(inner) => match &**inner {
                TType::Package(_) => self.prefix(inner),
                _ if self.full_names => format!("{}.this.", self.type_name_of(inner)),
                _ => String::new(),
            },
            TType::TermRef(..) | TType::LocalTerm(..) => format!("{}.", self.path(t)),
            t if self.is_module_class(t) => format!("{}.", self.path(t)),
            t if self.term_param(t).is_some() => format!("{}.", self.path(t)),
            other => format!("{}#", self.ty(other)),
        }
    }

    fn is_module_class(&self, t: &TType) -> bool {
        match t {
            TType::TypeRef(_, n) => self.file.is_object_class(*n),
            TType::LocalType(addr, _) => self.decoder.name_at(*addr).map_or(false, |n| self.file.is_object_class(n)),
            _ => false,
        }
    }

    fn type_name_of(&mut self, t: &TType) -> String {
        match t {
            TType::TypeRef(_, n) => self.name(self.file.source_name(*n)),
            TType::LocalType(addr, _) => match self.decoder.name_at(*addr) {
                Some(n) => self.name(self.file.source_name(n)),
                None => self.local_name(*addr),
            },
            other => self.ty(other),
        }
    }

    /// A stable path: `a.b`, `C.this`, an object by its class.
    fn path(&mut self, t: &TType) -> String {
        match t {
            TType::TermRef(prefix, n) => format!("{}{}{}", self.prefix(prefix), self.name(*n), self.signature_of(*n)),
            TType::LocalTerm(addr, prefix) => {
                let p = prefix.as_ref().map_or(String::new(), |p| self.prefix(p));
                let signature = self.local_signature(*addr);
                format!("{}{}{}", p, self.local_name(*addr), signature)
            }
            TType::TypeRef(prefix, n) if self.file.is_object_class(*n) => {
                format!("{}{}", self.prefix(prefix), self.name(self.file.source_name(*n)))
            }
            TType::LocalType(_, prefix) if self.is_module_class(t) => {
                let p = prefix.as_ref().map_or(String::new(), |p| self.prefix(p));
                format!("{}{}", p, self.type_name_of(t))
            }
            TType::This(inner) => format!("{}.this", self.type_name_of(inner)),
            TType::Package(p) => self.name(*p),
            other => match self.term_param(other) {
                Some(n) => self.name(n),
                None => self.ty(other),
            },
        }
    }

    /// The name of a method type's parameter that `t` refers to, a path.
    fn term_param(&self, t: &TType) -> Option<NameRef> {
        let TType::ParamRef(binder, num) = t else { return None };
        self.binders.iter().rev().find(|(b, ..)| b == binder).filter(|(.., term)| *term).and_then(|(_, ns, _)| ns.get(*num as usize).copied())
    }

    fn args(&mut self, args: &[TType]) -> String {
        args.iter().map(|a| self.ty(a)).collect::<Vec<_>>().join(", ")
    }

    fn bounds(&mut self, lo: &TType, hi: &TType) -> String {
        let mut s = String::new();
        if !self.is_scala_type(lo, "Nothing") {
            s.push_str(&format!(" >: {}", self.ty(lo)));
        }
        if !self.is_scala_type(hi, "Any") {
            s.push_str(&format!(" <: {}", self.ty(hi)));
        }
        s
    }

    /// A type in an argument position of a function arrow or an infix type.
    fn atom(&mut self, t: &TType) -> String {
        let s = self.ty(t);
        let compound = match t {
            TType::And(..) | TType::Or(..) | TType::ByName(_) | TType::Lambda { .. } | TType::Match { .. } => true,
            TType::Applied(tycon, _) => self.scala_name(tycon).and_then(super::shapes::function_class).is_some(),
            _ => false,
        };
        if compound {
            format!("({})", s)
        } else {
            s
        }
    }

    pub fn ty(&mut self, t: &TType) -> String {
        match t {
            TType::Package(p) => self.name(*p),
            TType::TypeRef(prefix, n) => {
                if self.file.is_object_class(*n) {
                    return format!("{}.type", self.path(t));
                }
                format!("{}{}", self.prefix(prefix), self.name(*n))
            }
            TType::LocalType(addr, prefix) => {
                if self.is_module_class(t) {
                    return format!("{}.type", self.path(t));
                }
                let p = prefix.as_ref().map_or(String::new(), |p| self.prefix(p));
                format!("{}{}", p, self.local_name(*addr))
            }
            TType::TermRef(..) | TType::LocalTerm(..) | TType::This(_) => format!("{}.type", self.path(t)),
            TType::Applied(tycon, args) => {
                if let Some(n) = self.scala_name(tycon) {
                    let arrow = match super::shapes::function_class(n) {
                        Some((true, arity)) if arity + 1 == args.len() => Some("?=>"),
                        Some((false, arity)) if arity + 1 == args.len() => Some("=>"),
                        _ => None,
                    };
                    if let (Some(arrow), Some((ret, params))) = (arrow, args.split_last()) {
                        let ps = match params {
                            [one] if !matches!(one, TType::Applied(c, _) if self.scala_name(c).map_or(false, |n| n.starts_with("Tuple"))) => {
                                self.atom(one)
                            }
                            _ => format!("({})", self.args(params)),
                        };
                        return format!("{} {} {}", ps, arrow, self.ty(ret));
                    }
                    if n.starts_with("Tuple") && n["Tuple".len()..].parse::<u32>().is_ok() && args.len() > 1 {
                        return format!("({})", self.args(args));
                    }
                    if n == "<repeated>" && args.len() == 1 {
                        return format!("{}*", self.atom(&args[0]));
                    }
                }
                format!("{}[{}]", self.ty(tycon), self.args(args))
            }
            TType::Bounds(lo, hi) => {
                let b = self.bounds(lo, hi);
                if b.is_empty() {
                    "?".to_string()
                } else {
                    format!("?{}", b)
                }
            }
            TType::Alias(a) => format!("= {}", self.ty(a)),
            TType::BoundedAlias(b, a) => format!("{} = {}", self.ty(b), self.ty(a)),
            TType::And(a, b) => format!("{} & {}", self.atom(a), self.atom(b)),
            TType::Or(a, b) => format!("{} | {}", self.atom(a), self.atom(b)),
            TType::ByName(u) => format!("=> {}", self.ty(u)),
            TType::Annotated(u, annot) => {
                let a = self.type_name_of(annot);
                if a == "Repeated" {
                    if let TType::Applied(_, args) = &**u {
                        if args.len() == 1 {
                            return format!("{}*", self.atom(&args[0]));
                        }
                    }
                }
                format!("{} @{}", self.atom(u), a)
            }
            TType::Refined(parent, members) => {
                let ms: Vec<String> = members
                    .iter()
                    .map(|(n, info)| match info {
                        Some(TType::Alias(a)) => format!("type {} = {}", self.name(*n), self.ty(a)),
                        Some(TType::Bounds(lo, hi)) => format!("type {}{}", self.name(*n), self.bounds(lo, hi)),
                        Some(i @ TType::Lambda { .. }) => format!("def {}{}", self.name(*n), self.ty(i)),
                        Some(i) => format!("{}: {}", self.name(*n), self.ty(i)),
                        None => self.name(*n),
                    })
                    .collect();
                format!("{} {{ {} }}", self.ty(parent), ms.join("; "))
            }
            TType::Rec(_, u) => self.ty(u),
            TType::RecThis(_) => "this".to_string(),
            TType::Super(this, _) => format!("{}.super", self.path(this)),
            TType::Lambda { kind, binder, params, result } => {
                self.binders.push((*binder, params.iter().map(|p| p.name).collect(), *kind == LambdaKind::Method));
                let s = match kind {
                    LambdaKind::Type => format!("[{}] =>> {}", self.tparams(params), self.ty(result)),
                    LambdaKind::Poly => format!("[{}]{}", self.tparams(params), self.method_result(result)),
                    LambdaKind::Method => {
                        let ps: Vec<String> =
                            params.iter().map(|p| format!("{}: {}", self.name(p.name), self.ty(&p.info))).collect();
                        let using = if params.first().map_or(false, |p| p.flags.has(GIVEN)) { "using " } else { "" };
                        format!("({}{}){}", using, ps.join(", "), self.method_result(result))
                    }
                };
                self.binders.pop();
                s
            }
            TType::ParamRef(binder, num) => {
                if let Some(n) = self.term_param(t) {
                    return format!("{}.type", self.name(n));
                }
                match self.binders.iter().rev().find(|(b, ..)| b == binder).and_then(|(_, ns, _)| ns.get(*num as usize)) {
                    Some(&n) => self.name(n),
                    None => format!("<param {}>", num),
                }
            }
            TType::Match { scrutinee, cases, .. } => format!("{} match {{ {} cases }}", self.atom(scrutinee), cases.len()),
            TType::MatchCase(p, b) => format!("case {} => {}", self.ty(p), self.ty(b)),
            TType::Flexible(u) => format!("({})?", self.ty(u)),
            TType::Const(c) => self.constant(c),
            TType::Unknown(crate::scala2::UNSUPPORTED) => "<a Scala 2 type teq does not read>".to_string(),
            TType::Unknown(tag) => format!("<tag {}>", tag),
        }
    }

    fn method_result(&mut self, result: &TType) -> String {
        match result {
            TType::Lambda { kind: LambdaKind::Method | LambdaKind::Poly, .. } => self.ty(result),
            _ => format!(": {}", self.ty(result)),
        }
    }

    fn constant(&mut self, c: &Const) -> String {
        match c {
            Const::Unit => "()".to_string(),
            Const::Bool(b) => b.to_string(),
            Const::Byte(v) | Const::Short(v) | Const::Int(v) => v.to_string(),
            Const::Char(v) => match char::from_u32(*v) {
                Some('\'') => "'\\''".to_string(),
                Some('\\') => "'\\\\'".to_string(),
                Some('\n') => "'\\n'".to_string(),
                Some('\t') => "'\\t'".to_string(),
                Some(c) if (' '..='~').contains(&c) => format!("'{}'", c),
                _ => format!("'\\u{:04x}'", v),
            },
            Const::Long(v) => format!("{}L", v),
            Const::Float(bits) => format!("{}f", f32::from_bits(*bits)),
            Const::Double(bits) => format!("{}d", f64::from_bits(*bits)),
            Const::Str(n) => format!("{:?}", self.name(*n)),
            Const::Null => "null".to_string(),
            Const::Class(t) => format!("classOf[{}]", self.ty(t)),
        }
    }

    fn tparam(&mut self, p: &TParam) -> String {
        let variance = if p.flags.has(COVARIANT) {
            "+"
        } else if p.flags.has(CONTRAVARIANT) {
            "-"
        } else {
            ""
        };
        let name = self.name(p.name);
        let rest = match &p.info {
            TType::Bounds(lo, hi) if self.is_scala_type(lo, "Nothing") && matches!(&**hi, TType::Lambda { kind: LambdaKind::Type, .. }) => {
                let TType::Lambda { binder, params, result, .. } = &**hi else { unreachable!() };
                self.binders.push((*binder, params.iter().map(|p| p.name).collect(), false));
                let inner = self.tparams(params);
                let b = if self.is_scala_type(result, "Any") { String::new() } else { format!(" <: {}", self.ty(result)) };
                self.binders.pop();
                format!("[{}]{}", inner, b)
            }
            TType::Bounds(lo, hi) => self.bounds(lo, hi),
            TType::Lambda { kind: LambdaKind::Type, binder, params, result } => {
                self.binders.push((*binder, params.iter().map(|p| p.name).collect(), false));
                let inner = self.tparams(params);
                let b = match &**result {
                    TType::Bounds(lo, hi) => self.bounds(lo, hi),
                    other => format!(" {}", self.ty(other)),
                };
                self.binders.pop();
                format!("[{}]{}", inner, b)
            }
            other => format!(" {}", self.ty(other)),
        };
        format!("{}{}{}", variance, name, rest)
    }

    fn tparams(&mut self, ps: &[TParam]) -> String {
        ps.iter().map(|p| self.tparam(p)).collect::<Vec<_>>().join(", ")
    }

    fn clause(&mut self, c: &Clause) -> String {
        match c {
            Clause::Types(ps) => format!("[{}]", self.tparams(ps)),
            Clause::Terms(ps) => {
                let lead = match ps.first() {
                    Some(p) if p.flags.has(GIVEN) => "using ",
                    Some(p) if p.flags.has(IMPLICIT) => "implicit ",
                    _ => "",
                };
                let items: Vec<String> = ps
                    .iter()
                    .map(|p| {
                        let mut s = String::new();
                        for (tag, word) in [(ERASED, "erased "), (INLINE, "inline ")] {
                            if p.flags.has(tag) {
                                s.push_str(word);
                            }
                        }
                        s.push_str(&format!("{}: {}", self.name(p.name), self.ty(&p.ty)));
                        if p.flags.has(HASDEFAULT) {
                            s.push_str(" = <default>");
                        }
                        s
                    })
                    .collect();
                format!("({}{})", lead, items.join(", "))
            }
        }
    }

    /// A constructor clause, each parameter with what its accessor says: `val`, `var`, access.
    fn ctor_clause(&mut self, c: &Clause, accessors: &[Param]) -> String {
        let plain = self.clause(c);
        let Clause::Terms(ps) = c else { return plain };
        let mut text = plain;
        for p in ps {
            let Some(a) = accessors.iter().find(|a| a.name == p.name) else { continue };
            if a.flags.has(PRIVATE) && a.flags.has(LOCAL) {
                continue;
            }
            let mut lead = String::new();
            for (tag, word) in [(PRIVATE, "private "), (PROTECTED, "protected ")] {
                if a.flags.has(tag) {
                    lead.push_str(word);
                }
            }
            lead.push_str(if a.flags.has(MUTABLE) { "var " } else { "val " });
            let name = self.name(p.name);
            for open in ["(", ", ", "using ", "implicit "] {
                let from = format!("{}{}: ", open, name);
                if let Some(at) = text.find(&from) {
                    text.insert_str(at + open.len(), &lead);
                    break;
                }
            }
        }
        text
    }

    fn mods(&mut self, m: &Mods, skip: &[u8]) -> String {
        let mut s = String::new();
        for a in &m.annots {
            s.push_str(&format!("@{} ", self.type_name_of(a)));
        }
        for &(tag, marker) in MARKERS {
            if m.flags.has(tag) {
                s.push_str(&format!("<{}> ", marker));
            }
        }
        for &(tag, word) in KEYWORDS {
            if !m.flags.has(tag) || skip.contains(&tag) {
                continue;
            }
            s.push_str(word);
            if matches!(tag, PRIVATE | PROTECTED) {
                if m.flags.has(LOCAL) {
                    s.push_str("[this]");
                } else if let Some(w) = &m.within {
                    let scope = self.type_name_of(w);
                    s.push_str(&format!("[{}]", scope.rsplit('.').next().unwrap_or("")));
                }
            }
            s.push(' ');
        }
        s
    }

    fn indent(&mut self, depth: usize) {
        for _ in 0..depth {
            self.out.push_str("  ");
        }
    }

    pub fn def(&mut self, sig: &DefSig, depth: usize) {
        self.indent(depth);
        let mut line = self.def_head(sig);
        match sig.body {
            Some(addr) if sig.mods.flags.has(INLINE) => line.push_str(&format!(" = <body@{}>", addr)),
            Some(_) => line.push_str(" = ..."),
            None => {}
        }
        self.out.push_str(&line);
        self.out.push('\n');
    }

    /// Modifiers, keyword, name, clauses and result type, without the body.
    pub fn def_head(&mut self, sig: &DefSig) -> String {
        let f = sig.mods.flags;
        let mut line = self.mods(&sig.mods, if f.has(GIVEN) { &[LAZY, FINAL] } else { &[] });
        let name = self.name(sig.name);
        // A right-associative extension method is stored with its first explicit clause in front
        // of the receiver: `extension (v: Vec) def +:(x: Int)` as `+:(x: Int)(v: Vec)`.
        let mut reordered = sig.clauses.clone();
        if f.has(EXTENSION) && name.ends_with(':') {
            let explicit: Vec<usize> = reordered
                .iter()
                .enumerate()
                .filter(|(_, c)| matches!(c, Clause::Terms(ps) if !ps.first().map_or(false, |p| p.flags.has(GIVEN))))
                .map(|(i, _)| i)
                .collect();
            if let [first, second, ..] = explicit[..] {
                if second == first + 1 {
                    reordered.swap(first, second);
                }
            }
        }
        let mut clauses = &reordered[..];
        if f.has(EXTENSION) {
            // The receiver is the first term clause; what stands before it belongs to the extension.
            let split = clauses.iter().position(|c| matches!(c, Clause::Terms(ps) if !ps.first().map_or(false, |p| p.flags.has(GIVEN)))).map_or(0, |i| i + 1);
            line.push_str("extension ");
            for c in &clauses[..split] {
                line.push_str(&self.clause(c));
            }
            line.push(' ');
            clauses = &clauses[split..];
        }
        let keyword = if f.has(GIVEN) {
            "given"
        } else if sig.tag == DEFDEF {
            "def"
        } else if f.has(ENUM) && f.has(CASE) {
            "case"
        } else if f.has(MUTABLE) {
            "var"
        } else {
            "val"
        };
        line.push_str(&format!("{} {}", keyword, name));
        for c in clauses {
            line.push_str(&self.clause(c));
        }
        line.push_str(&format!(": {}", self.ty(&sig.ret)));
        line
    }

    pub fn type_def(&mut self, sig: &TypeDefSig, depth: usize) {
        self.indent(depth);
        let mut line = self.mods(&sig.mods, &[]);
        line.push_str(&format!("type {}", self.name(sig.name)));
        let mut rhs = &sig.rhs;
        let mut pushed = false;
        if let TType::Lambda { kind: LambdaKind::Type, binder, params, result } = rhs {
            self.binders.push((*binder, params.iter().map(|p| p.name).collect(), false));
            pushed = true;
            line.push_str(&format!("[{}]", self.tparams(params)));
            rhs = result;
        }
        if let Some(TType::Bounds(lo, hi)) = &sig.opaque_bounds {
            line.push_str(&self.bounds(lo, hi));
        }
        match rhs {
            TType::Bounds(lo, hi) => line.push_str(&self.bounds(lo, hi)),
            TType::Alias(a) => line.push_str(&format!(" = {}", self.ty(a))),
            other => line.push_str(&format!(" = {}", self.ty(other))),
        }
        if pushed {
            self.binders.pop();
        }
        self.out.push_str(&line);
        self.out.push('\n');
    }

    /// The line that opens a class: modifiers, keyword, name, type parameters, constructor
    /// clauses, parents and self type, up to and including the `{`.
    fn class_head(
        &mut self,
        name: NameRef,
        mods: &Mods,
        tparams: &[TParam],
        ctor: Option<&DefSig>,
        accessors: &[Param],
        parents: Vec<String>,
        self_type: Option<&(NameRef, TType)>,
    ) -> String {
        let f = mods.flags;
        let keyword = if f.has(OBJECT) && f.has(GIVEN) {
            "given object"
        } else if f.has(OBJECT) {
            "object"
        } else if f.has(TRAIT) {
            "trait"
        } else if f.has(ENUM) && f.has(CASE) {
            "case"
        } else if f.has(ENUM) {
            "enum"
        } else if f.has(CASE) {
            "case class"
        } else {
            "class"
        };
        let skip: &[u8] = if f.has(OBJECT) { &[FINAL, LAZY] } else if f.has(TRAIT) { &[ABSTRACT] } else { &[] };
        let mut line = self.mods(mods, skip);
        line.push_str(&format!("{} {}", keyword, self.name(self.file.source_name(name))));
        if !tparams.is_empty() {
            line.push_str(&format!("[{}]", self.tparams(tparams)));
        }
        if let Some(ctor) = ctor {
            if !f.has(OBJECT) {
                let ctor_mods = self.mods(&ctor.mods, &[]);
                if !ctor_mods.is_empty() {
                    line.push_str(&format!(" {}", ctor_mods.trim_end()));
                }
                // The constructor repeats the type parameters of the class.
                for c in ctor.clauses.iter().filter(|c| matches!(c, Clause::Terms(_))) {
                    let text = self.ctor_clause(c, accessors);
                    if !(f.has(TRAIT) && text == "()") {
                        line.push_str(&text);
                    }
                }
            }
        }
        if !parents.is_empty() {
            line.push_str(&format!(" extends {}", parents.join(", ")));
        }
        line.push_str(" {");
        if let Some((n, t)) = self_type.filter(|_| !f.has(OBJECT)) {
            line.push_str(&format!(" {}: {} =>", self.name(*n), self.ty(t)));
        }
        line
    }

    /// `module_val` holds the modifiers of the val of an object, which is where `implicit` and
    /// `given` stand: the class of an object carries neither.
    pub fn class(&mut self, entry: &Entry, module_val: Flags, depth: usize) {
        let mut sig = self.decoder.class_sig(entry.addr);
        for tag in [IMPLICIT, GIVEN] {
            if module_val.has(tag) {
                sig.mods.flags.set(tag);
            }
        }
        self.indent(depth);
        let parents: Vec<String> = sig.parents.iter().map(|p| self.ty(p)).collect();
        let line = self.class_head(sig.name, &sig.mods, &sig.tparams, sig.ctor.as_ref(), &sig.accessors, parents, sig.self_type.as_ref());
        self.out.push_str(&line);
        self.out.push('\n');
        for &addr in &sig.index.exports {
            let e = self.decoder.export_sig(addr);
            self.indent(depth + 1);
            let sels: Vec<String> = e
                .selectors
                .iter()
                .map(|(n, to)| {
                    let from = match self.file.simple(*n) {
                        Some("_") => "*".to_string(),
                        Some("") => "given".to_string(),
                        _ => self.name(*n),
                    };
                    match to {
                        Some(to) => format!("{} as {}", from, self.name(*to)),
                        None => from,
                    }
                })
                .collect();
            let path = self.path(&e.path);
            self.out.push_str(&format!("export {}.{{{}}}\n", path, sels.join(", ")));
        }
        self.members(&sig.index.members, depth + 1);
        self.indent(depth);
        self.out.push_str("}\n");
    }

    /// Prints definitions of one scope; an object finds the val that goes with its class there.
    pub fn members(&mut self, members: &[Entry], depth: usize) {
        for m in members {
            let module_val = if m.tag == TYPEDEF && m.is_class && self.file.is_object_class(m.name) {
                let source = self.file.source_name(m.name);
                members
                    .iter()
                    .find(|v| v.tag == VALDEF && v.flags.has(OBJECT) && v.name == source)
                    .map_or(Flags::default(), |v| v.flags)
            } else {
                Flags::default()
            };
            self.member(m, module_val, depth);
        }
    }

    fn member(&mut self, m: &Entry, module_val: Flags, depth: usize) {
        match m.tag {
            TYPEDEF if m.is_class => self.class(m, module_val, depth),
            TYPEDEF => {
                let sig = self.decoder.type_def_sig(m.addr);
                self.type_def(&sig, depth);
            }
            // The val of an object says nothing its class does not.
            VALDEF if m.flags.has(OBJECT) => {}
            _ => {
                let sig = self.decoder.def_sig(m.addr);
                self.def(&sig, depth);
            }
        }
    }
}

/// The name kinds a file's name table holds, for `teq tasty --names`.
pub fn name_kind(n: &TName) -> &'static str {
    match n {
        TName::Simple(_) => "simple",
        TName::Qualified(..) => "qualified",
        TName::Expanded(..) => "expanded",
        TName::ExpandPrefix(..) => "expandprefix",
        TName::Unique { .. } => "unique",
        TName::DefaultGetter(..) => "defaultgetter",
        TName::SuperAccessor(_) => "superaccessor",
        TName::InlineAccessor(_) => "inlineaccessor",
        TName::ObjectClass(_) => "objectclass",
        TName::BodyRetainer(_) => "bodyretainer",
        TName::Signed { target: None, .. } => "signed",
        TName::Signed { .. } => "targetsigned",
    }
}

// ---- terms ----

/// Whether an operand of an infix application prints without parentheses.
fn is_atomic_term(t: &Term) -> bool {
    match &t.kind {
        TermKind::Apply(f, args) => !(args.len() == 1 && is_infix_callee(f)),
        TermKind::Path(_)
        | TermKind::Const(_)
        | TermKind::Ident(..)
        | TermKind::Select(..)
        | TermKind::SelectIn(..)
        | TermKind::QualThis(_)
        | TermKind::New(_)
        | TermKind::TypeApply(..)
        | TermKind::Typed(..)
        | TermKind::Block(..)
        | TermKind::Repeated(..)
        | TermKind::Quote { .. }
        | TermKind::Splice { .. }
        | TermKind::Super(..)
        | TermKind::SelectOuter { .. }
        | TermKind::Unknown(..)
        | TermKind::TooDeep(_)
        | TermKind::Cycle(_) => true,
        _ => false,
    }
}

fn is_infix_callee(f: &Term) -> bool {
    matches!(&f.kind, TermKind::Select(_, _) | TermKind::SelectIn(..))
}

fn is_symbolic(name: &str) -> bool {
    !name.is_empty() && !name.chars().any(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

impl<'a> Printer<'a> {
    fn is_wildcard(&self, t: &Term) -> bool {
        matches!(&t.kind, TermKind::Ident(n, _) if self.file.simple(*n) == Some("_"))
    }

    fn member_name(&self, n: NameRef) -> String {
        self.name(self.file.source_name(n))
    }

    /// A signed name's signature, `{(params): result}`, where `--signatures` asks for it.
    fn signature_of(&self, n: NameRef) -> String {
        match self.file.names.get(n as usize) {
            Some(TName::Signed { .. }) if SIGNATURES.load(std::sync::atomic::Ordering::Relaxed) => {
                let text = super::dump::name_text(self.file, n);
                text.split_once('(').map_or(String::new(), |(_, rest)| format!("{{({}}}", rest))
            }
            _ => String::new(),
        }
    }

    fn indent_str(depth: usize) -> String {
        "  ".repeat(depth)
    }

    /// A path in a term position: a term reference as a path, a type tree as a type.
    fn path_term(&mut self, t: &TType) -> String {
        match t {
            TType::TermRef(..) | TType::LocalTerm(..) | TType::This(_) | TType::Package(_) => self.path(t),
            TType::Const(c) => self.constant(c),
            other => self.ty(other),
        }
    }

    fn operand(&mut self, t: &Term, depth: usize) -> String {
        let s = self.term(t, depth);
        if is_atomic_term(t) {
            s
        } else {
            format!("({})", s)
        }
    }

    fn term_list(&mut self, ts: &[Term], depth: usize) -> String {
        ts.iter().map(|t| self.term(t, depth)).collect::<Vec<_>>().join(", ")
    }

    /// The extractor of an `unapply` call as the pattern names it: `Some(x)`, not `Some.unapply(x)`.
    fn extractor(&mut self, f: &Term, depth: usize) -> String {
        match &f.kind {
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) if matches!(self.file.simple(self.file.source_name(*n)), Some("unapply" | "unapplySeq")) => {
                self.term(q, depth)
            }
            TermKind::TypeApply(g, targs, ..) => format!("{}[{}]", self.extractor(g, depth), self.args(targs)),
            _ => self.term(f, depth),
        }
    }

    /// The class type of `new C.<init>`, the callee of a constructor call.
    /// The parameters of the method a reference by address names, `{[1](String)}`, where
    /// `--signatures` asks for them: which of two overloads the reference is.
    fn local_signature(&mut self, addr: Addr) -> String {
        if !SIGNATURES.load(std::sync::atomic::Ordering::Relaxed) || self.decoder.tag_at(addr) != DEFDEF {
            return String::new();
        }
        let sig = self.decoder.def_sig(addr);
        let clauses: Vec<String> = sig
            .clauses
            .iter()
            .map(|c| match c {
                Clause::Types(tps) => format!("[{}]", tps.len()),
                Clause::Terms(ps) => format!("({})", ps.iter().map(|p| self.ty(&p.ty)).collect::<Vec<_>>().join(", ")),
            })
            .collect();
        format!("{{{}}}", clauses.join(""))
    }

    /// The signature of the constructor a `new` selects, where `--signatures` asks for it.
    fn constructor_signature(&self, t: &Term) -> String {
        match &t.kind {
            TermKind::Select(_, n) | TermKind::SelectIn(_, n, ..) => self.signature_of(*n),
            TermKind::TypeApply(f, ..) => self.constructor_signature(f),
            _ => String::new(),
        }
    }

    fn constructed<'t>(&self, t: &'t Term) -> Option<&'t TType> {
        match &t.kind {
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) if self.file.simple(self.file.source_name(*n)) == Some("<init>") => match &q.kind {
                TermKind::New(t) => Some(t),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn term(&mut self, t: &Term, depth: usize) -> String {
        if let Some(c) = self.constructed(t) {
            let signature = self.constructor_signature(t);
            return format!("new {}{}", self.ty(c), signature);
        }
        match &t.kind {
            TermKind::Path(t) => self.path_term(t),
            TermKind::Const(c) => self.constant(c),
            // The signature an identifier's type names it by, `TermRef(prefix, f(..))`.
            TermKind::Ident(n, ty) => match ty {
                TType::TermRef(_, signed) => format!("{}{}", self.name(*n), self.signature_of(*signed)),
                _ => self.name(*n),
            },
            TermKind::Select(q, n) => format!("{}.{}{}", self.operand(q, depth), self.member_name(*n), self.signature_of(*n)),
            // With `--full` the class the member is resolved in follows the name.
            TermKind::SelectIn(q, n, owner, _) => {
                let at = if self.full_names { format!("@{}", self.type_name_of(owner)) } else { String::new() };
                let signature = self.signature_of(*n);
                format!("{}.{}{}{}", self.operand(q, depth), self.member_name(*n), signature, at)
            }
            TermKind::QualThis(t) => format!("{}.this", self.type_name_of(t)),
            TermKind::New(t) => format!("new {}", self.ty(t)),
            TermKind::Throw(e) => format!("throw {}", self.term(e, depth)),
            TermKind::NamedArg(n, e) => format!("{} = {}", self.name(*n), self.term(e, depth)),
            TermKind::Apply(f, args) | TermKind::ApplySigPoly(f, _, args) => {
                if let (TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..), [arg]) = (&f.kind, &args[..]) {
                    let name = self.member_name(*n);
                    if is_symbolic(&name) {
                        let signature = self.signature_of(*n);
                        return format!("{} {}{} {}", self.operand(q, depth), name, signature, self.operand(arg, depth));
                    }
                }
                format!("{}({})", self.operand(f, depth), self.term_list(args, depth))
            }
            TermKind::TypeApply(f, targs, ..) => {
                if let Some(t) = self.constructed(f) {
                    let applied = match t {
                        TType::Applied(..) => self.ty(t),
                        TType::Lambda { kind: LambdaKind::Type, result, .. } if matches!(&**result, TType::Applied(..)) => {
                            let TType::Applied(tycon, _) = &**result else { unreachable!() };
                            format!("{}[{}]", self.ty(tycon), self.args(targs))
                        }
                        _ => format!("{}[{}]", self.ty(t), self.args(targs)),
                    };
                    let signature = self.constructor_signature(f);
                    return format!("new {}{}", applied, signature);
                }
                format!("{}[{}]", self.operand(f, depth), self.args(targs))
            }
            TermKind::Super(this, mixin) => {
                let qual = match &this.kind {
                    TermKind::QualThis(t) => format!("{}.", self.type_name_of(t)),
                    _ => String::new(),
                };
                let mix = mixin.as_ref().map_or(String::new(), |m| format!("[{}]", self.ty(m)));
                format!("{}super{}", qual, mix)
            }
            TermKind::Typed(e, ty, ..) => match &e.kind {
                // The ascription of a vararg or a pattern says what the tree inside it does.
                TermKind::Repeated(..) | TermKind::Unapply { .. } => self.term(e, depth),
                _ if self.is_wildcard(e) => format!("_: {}", self.ty(ty)),
                _ => format!("({}: {})", self.term(e, depth), self.ty(ty)),
            },
            TermKind::Assign(a, b) => format!("{} = {}", self.term(a, depth), self.term(b, depth)),
            TermKind::Block(stats, expr) => self.block(stats, expr, depth),
            TermKind::If { inline, cond, then, els } => format!(
                "{}if {} then {} else {}",
                if *inline { "inline " } else { "" },
                self.term(cond, depth),
                self.term(then, depth),
                self.term(els, depth)
            ),
            TermKind::Lambda(meth, ty) => {
                let t = ty.as_ref().map_or(String::new(), |t| format!(": {}", self.ty(t)));
                format!("closure({}){}", self.term(meth, depth), t)
            }
            TermKind::Match { kind, selector, cases } => {
                let sel = selector.as_ref().map_or(String::new(), |s| format!("{} ", self.operand(s, depth)));
                let head = match kind {
                    MatchKind::Plain => format!("{}match", sel),
                    MatchKind::Inline => format!("inline {}match", sel),
                    MatchKind::Implicit => "inline match".to_string(),
                    MatchKind::Sub => format!("{}submatch", sel),
                };
                format!("{} {}", head, self.cases(cases, depth))
            }
            TermKind::Try { body, cases, finalizer } => {
                let mut s = format!("try {}", self.term(body, depth));
                if !cases.is_empty() {
                    s.push_str(&format!(" catch {}", self.cases(cases, depth)));
                }
                if let Some(f) = finalizer {
                    s.push_str(&format!(" finally {}", self.term(f, depth)));
                }
                s
            }
            TermKind::Return { expr, .. } => match expr {
                Some(e) => format!("return {}", self.term(e, depth)),
                None => "return".to_string(),
            },
            TermKind::While(c, b) => format!("while {} do {}", self.term(c, depth), self.term(b, depth)),
            TermKind::Inlined { expansion, call, bindings, .. } => {
                let head = match call {
                    Some(c) => format!("inlined({})", self.term(c, depth)),
                    None => "inlined".to_string(),
                };
                if bindings.is_empty() {
                    return format!("{} {}", head, self.term(expansion, depth));
                }
                format!("{} {}", head, self.block(bindings, expansion, depth))
            }
            TermKind::Repeated(_, elems) => format!("[{}]*", self.term_list(elems, depth)),
            TermKind::SelectOuter { levels, qual, .. } => {
                let n = if *levels > 1 { levels.to_string() } else { String::new() };
                format!("{}.<outer{}>", self.operand(qual, depth), n)
            }
            TermKind::Quote { body, .. } => match &body.kind {
                TermKind::Path(t) if !matches!(t, TType::TermRef(..) | TType::LocalTerm(..) | TType::This(_)) => {
                    format!("'[{}]", self.ty(t))
                }
                _ => format!("'{{ {} }}", self.term(body, depth)),
            },
            TermKind::Splice { expr, .. } => format!("${{ {} }}", self.term(expr, depth)),
            TermKind::QuotePattern { body, bindings, .. } => {
                if bindings.is_empty() {
                    return format!("'{{ {} }}", self.term(body, depth));
                }
                format!("'{}", self.block(bindings, body, depth))
            }
            TermKind::SplicePattern { pat, targs, args, .. } => {
                let mut s = format!("${{ {} }}", self.term(pat, depth));
                if !targs.is_empty() {
                    s.push_str(&format!("[{}]", self.args(targs)));
                }
                if !args.is_empty() {
                    s.push_str(&format!("({})", self.term_list(args, depth)));
                }
                s
            }
            TermKind::Bind { name, body, flags, .. } => {
                let n = format!("{}{}", if flags.has(GIVEN) { "given " } else { "" }, self.name(*name));
                match &body.kind {
                    _ if self.is_wildcard(body) => n,
                    TermKind::Typed(inner, ty, ..) if self.is_wildcard(inner) => format!("{}: {}", n, self.ty(ty)),
                    _ => format!("{} @ {}", n, self.operand(body, depth)),
                }
            }
            TermKind::Alternative(alts) => alts.iter().map(|a| self.operand(a, depth)).collect::<Vec<_>>().join(" | "),
            TermKind::Unapply { fun, implicits, pats, .. } => {
                let mut s = format!("{}({})", self.extractor(fun, depth), self.term_list(pats, depth));
                if !implicits.is_empty() {
                    s.push_str(&format!("(using {})", self.term_list(implicits, depth)));
                }
                s
            }
            TermKind::Elided(t) => format!("<elided: {}>", self.ty(t)),
            TermKind::Hole { idx, .. } => format!("<hole {}>", idx),
            TermKind::Unknown(tag, addr) => format!("<tag {} @{}>", tag, addr),
            TermKind::TooDeep(addr) => format!("<too deep @{}>", addr),
            TermKind::Cycle(addr) => format!("<shared @{}>", addr),
        }
    }

    fn cases(&mut self, cases: &[Case], depth: usize) -> String {
        let mut s = String::from("{\n");
        for c in cases {
            s.push_str(&Self::indent_str(depth + 1));
            s.push_str("case ");
            s.push_str(&self.term(&c.pat, depth + 1));
            if let Some(g) = &c.guard {
                s.push_str(&format!(" if {}", self.term(g, depth + 1)));
            }
            s.push_str(&format!(" => {}\n", self.term(&c.body, depth + 1)));
        }
        s.push_str(&Self::indent_str(depth));
        s.push('}');
        s
    }

    /// A block; the closure over a local method that it defines prints as the lambda it is.
    fn block(&mut self, stats: &[Stat], expr: &Term, depth: usize) -> String {
        if let ([Stat::Def(sig, Some(body))], TermKind::Lambda(meth, _)) = (stats, &expr.kind) {
            if matches!(&meth.kind, TermKind::Path(TType::LocalTerm(addr, _)) if *addr == sig.addr) {
                let params: Vec<String> = sig.clauses.iter().map(|c| self.clause(c)).collect();
                return format!("{} => {}", params.join(""), self.term(body, depth));
            }
        }
        if stats.is_empty() {
            return format!("{{ {} }}", self.term(expr, depth));
        }
        let mut s = String::from("{\n");
        for st in stats {
            s.push_str(&self.stat(st, depth + 1));
        }
        s.push_str(&Self::indent_str(depth + 1));
        s.push_str(&self.term(expr, depth + 1));
        s.push('\n');
        s.push_str(&Self::indent_str(depth));
        s.push('}');
        s
    }

    /// One statement as lines, each indented by `depth` and ended with a newline.
    pub fn stat(&mut self, s: &Stat, depth: usize) -> String {
        let mut out = Self::indent_str(depth);
        match s {
            Stat::Val(sig, rhs) | Stat::Def(sig, rhs) => {
                out.push_str(&self.def_head(sig));
                if let Some(rhs) = rhs {
                    out.push_str(&format!(" = {}", self.term(rhs, depth)));
                }
            }
            Stat::Type(sig) => {
                let saved = std::mem::take(&mut self.out);
                self.type_def(sig, 0);
                let line = std::mem::replace(&mut self.out, saved);
                out.push_str(line.trim_end());
            }
            Stat::Class(c) => {
                let saved = std::mem::take(&mut self.out);
                self.class_def(c, depth);
                let text = std::mem::replace(&mut self.out, saved);
                return text;
            }
            Stat::Import { path, selectors } => {
                let sels: Vec<String> = selectors
                    .iter()
                    .map(|(n, to)| {
                        let from = match self.file.simple(*n) {
                            Some("_") => "*".to_string(),
                            Some("") => "given".to_string(),
                            _ => self.name(*n),
                        };
                        match to {
                            Some(to) => format!("{} as {}", from, self.name(*to)),
                            None => from,
                        }
                    })
                    .collect();
                out.push_str(&format!("import {}.{{{}}}", self.path(path), sels.join(", ")));
            }
            Stat::Expr(t) => out.push_str(&self.term(t, depth)),
        }
        out.push('\n');
        out
    }

    /// A class with every body: the header, the constructor's statements, then the members.
    pub fn class_def(&mut self, c: &ClassDef, depth: usize) {
        let t = &c.template;
        self.indent(depth);
        let parents: Vec<String> = t
            .parents
            .iter()
            .map(|p| {
                let s = self.term(p, depth);
                s.strip_prefix("new ").map_or(s.clone(), str::to_string)
            })
            .collect();
        let line = self.class_head(c.name, &c.mods, &t.tparams, t.ctor.as_ref().map(|(sig, _)| sig), &t.params, parents, t.self_type.as_ref());
        self.out.push_str(&line);
        self.out.push('\n');
        if let Some((_, Some(body))) = &t.ctor {
            if !matches!(&body.kind, TermKind::Const(Const::Unit)) {
                let text = format!("{}<init> = {}\n", Self::indent_str(depth + 1), self.term(body, depth + 1));
                self.out.push_str(&text);
            }
        }
        for s in &t.stats {
            let text = self.stat(s, depth + 1);
            self.out.push_str(&text);
        }
        self.indent(depth);
        self.out.push_str("}\n");
    }
}
