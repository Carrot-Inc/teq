//! A Java source's declarations, as the reference's `JavaParsers.JavaParser` reads them (dotty 3.8.4): the package,
//! the imports, each type declaration with its modifiers, annotations, type parameters, parents and members; a
//! member's type, parameters and constant; method bodies, initializers and initializer blocks skipped (`methodBody`,
//! `varDecl`, `typeBodyDecls`), a `throws` clause read and dropped (`optThrows`).

use super::scanner::{Scanner, T};

/// A type as written.
#[derive(Clone, Debug)]
pub enum Ty {
    Prim(T),
    Void,
    /// A name, qualified or not, each segment with its type arguments and its place: `java.util.Map.Entry<K, V>`.
    Named(Vec<(String, Vec<Arg>, u32)>, u32),
    Array(Box<Ty>),
    /// `T...` as a parameter's type: an array, repeated.
    Repeated(Box<Ty>),
    Error,
}

#[derive(Clone, Debug)]
pub enum Arg {
    Ty(Ty),
    /// `?`, `? extends T`, `? super T`.
    Wild(Option<Box<Ty>>, Option<Box<Ty>>),
}

#[derive(Clone, Debug)]
pub struct TParam {
    pub name: String,
    /// Where its name stands.
    pub at: u32,
    /// `extends A & B`; none: `Object`.
    pub bounds: Vec<Ty>,
}

/// A literal a constant holds (`tryConstant`), before its conversion to the declared type.
#[derive(Clone, Debug, PartialEq)]
pub enum Lit {
    Bool(bool),
    Char(char),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Str(String),
}

/// An annotation kept: its type's name as written and where it stands (its arguments are read and not kept; one
/// with an argument the reference cannot read is dropped whole, `annotation`).
#[derive(Clone, Debug)]
pub struct Annot {
    pub name: Vec<String>,
}

pub const PUBLIC: u32 = 1;
pub const PROTECTED: u32 = 2;
pub const PRIVATE: u32 = 4;
pub const STATIC: u32 = 8;
pub const ABSTRACT: u32 = 16;
pub const FINAL: u32 = 32;
pub const DEFAULT: u32 = 64;
pub const SEALED: u32 = 128;
pub const NATIVE: u32 = 256;
pub const TRANSIENT: u32 = 512;
pub const VOLATILE: u32 = 1024;
pub const SYNCHRONIZED: u32 = 2048;
pub const NON_SEALED: u32 = 4096;

#[derive(Clone, Debug, Default)]
pub struct Mods {
    pub flags: u32,
    pub annots: Vec<Annot>,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub ty: Ty,
    pub name: String,
    /// Where its name stands.
    pub at: u32,
}

#[derive(Clone, Debug)]
pub enum Member {
    Field { mods: Mods, ty: Ty, name: String, constant: Option<Lit>, at: u32 },
    Method { mods: Mods, tparams: Vec<TParam>, ret: Ty, name: String, params: Vec<Param>, body: bool, annotation_default: bool, at: u32 },
    Ctor { mods: Mods, tparams: Vec<TParam>, params: Vec<Param> },
    Type(TypeDecl),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Class,
    Interface,
    Enum,
    Record,
    Annotation,
}

#[derive(Clone, Debug)]
pub struct TypeDecl {
    pub kind: Kind,
    pub name: String,
    /// Where the declaration starts, and where its name stands.
    pub start: u32,
    pub offset: u32,
    pub mods: Mods,
    pub tparams: Vec<TParam>,
    /// A class's `extends`; an interface's parents are its `interfaces`.
    pub superclass: Option<Ty>,
    pub interfaces: Vec<Ty>,
    pub permits: Vec<Ty>,
    pub members: Vec<Member>,
    /// An enum's constants by name.
    pub constants: Vec<(String, u32)>,
    /// A record's header.
    pub header: Vec<Param>,
}

#[derive(Clone, Debug)]
pub struct Import {
    pub is_static: bool,
    /// The names, the last `*` for an on-demand import.
    pub names: Vec<String>,
    /// Where each name stands in the source (a selector's place: the reference tells an import's fault at its last).
    pub at: Vec<u32>,
}

#[derive(Debug, Default)]
pub struct Unit {
    pub package: Vec<String>,
    pub imports: Vec<Import>,
    pub types: Vec<TypeDecl>,
    /// Where the first member outside any type stands (a compact source file, JEP 512): the reference drops the whole
    /// unit.
    pub compact: Option<u32>,
    pub errors: Vec<(u32, String)>,
}

struct P {
    s: Scanner,
    errors: Vec<(u32, String)>,
    /// The identifier `record` read as the keyword it is at a declaration's start (`adaptRecordIdentifier`).
    record: bool,
}

fn tok_str(t: T) -> &'static str {
    match t {
        T::Eof => "eof",
        T::Ident => "identifier",
        T::Semi => "';'",
        T::Comma => "','",
        T::LParen => "'('",
        T::RParen => "')'",
        T::LBrace => "'{'",
        T::RBrace => "'}'",
        T::LBracket => "'['",
        T::RBracket => "']'",
        T::Gt => "'>'",
        T::Lt => "'<'",
        T::Equals => "'='",
        T::Dot => "'.'",
        T::Class => "class",
        T::Interface => "interface",
        T::Import => "import",
        T::Package => "package",
        _ => "token",
    }
}

impl P {
    fn tok(&self) -> T {
        self.s.tok()
    }
    fn next(&mut self) {
        self.s.next_token();
    }
    fn err(&mut self, at: u32, msg: String) {
        if !self.errors.iter().any(|(o, _)| *o == at) {
            self.errors.push((at, msg));
        }
    }

    /// `skipAhead`: past a balanced parenthesised or braced run, to its closing token.
    fn skip_ahead(&mut self) {
        let mut parens = 0i32;
        let mut braces = 0i32;
        loop {
            match self.tok() {
                T::LParen => parens += 1,
                T::LBrace => braces += 1,
                _ => {}
            }
            self.next();
            match self.tok() {
                T::RParen => parens -= 1,
                T::RBrace => braces -= 1,
                _ => {}
            }
            if self.tok() == T::Eof || !(parens > 0 || braces > 0) {
                break;
            }
        }
    }

    fn skip_to(&mut self, stops: &[T]) {
        while !stops.contains(&self.tok()) && self.tok() != T::Eof {
            if self.tok() == T::LBrace {
                self.skip_ahead();
                self.accept(T::RBrace);
            } else if self.tok() == T::LParen {
                self.skip_ahead();
                self.accept(T::RParen);
            } else {
                self.next();
            }
        }
    }

    /// Where the parser stands: the token's offset and kind (a `>>` split into `>` stands at one offset as two).
    fn mark(&self) -> (u32, T) {
        (self.s.offset(), self.tok())
    }

    /// The guard of a loop whose round may consume nothing (an element that is none leaves its token, `{)` in an
    /// annotation's array): a round that ends where it began is past that token, an error there ("illegal start of
    /// <what>", unless one stands at it already). Every round of such a loop consumes a token or ends the input.
    fn progress(&mut self, began: (u32, T), what: &str) {
        if self.mark() == began && self.tok() != T::Eof {
            self.err(began.0, format!("illegal start of {}", what));
            self.next();
        }
    }

    fn accept(&mut self, t: T) -> u32 {
        let at = self.s.offset();
        if self.tok() != t {
            let msg = format!("{} expected but {} found.", tok_str(t), tok_str(self.tok()));
            self.err(at, msg);
        } else {
            self.next();
        }
        at
    }

    fn accept_closing_angle(&mut self) {
        match self.tok() {
            T::GtGtGtEq => self.s.set_tok(T::GtGtEq),
            T::GtGtGt => self.s.set_tok(T::GtGt),
            T::GtGtEq => self.s.set_tok(T::GtEq),
            T::GtGt => self.s.set_tok(T::Gt),
            T::GtEq => self.s.set_tok(T::Equals),
            _ => {
                self.accept(T::Gt);
            }
        }
    }

    fn ident(&mut self) -> String {
        if self.tok() == T::Ident {
            let n = self.s.name().to_string();
            self.next();
            n
        } else if matches!(self.tok(), T::Permits | T::Sealed) {
            // `permits` and `sealed` are contextual keywords (JLS 3.9): an identifier wherever one is read, as javac reads
            // them (a Java source's validity is javac's: the reference's scanner takes them as keywords everywhere)
            let n = if self.tok() == T::Permits { "permits" } else { "sealed" }.to_string();
            self.next();
            n
        } else {
            self.accept(T::Ident);
            "<error>".to_string()
        }
    }

    fn qual_id(&mut self) -> Vec<String> {
        let mut out = vec![self.ident()];
        while self.tok() == T::Dot && self.s.lookahead() == T::Ident {
            self.next();
            out.push(self.ident());
        }
        out
    }

    fn opt_array_brackets(&mut self, t: Ty) -> Ty {
        self.annotations();
        if self.tok() == T::LBracket {
            self.next();
            self.accept(T::RBracket);
            self.opt_array_brackets(Ty::Array(Box::new(t)))
        } else {
            t
        }
    }

    fn typ(&mut self) -> Ty {
        self.annotations();
        if self.tok() == T::Final {
            self.next();
        }
        let t = if self.tok() == T::Ident {
            let at = self.s.offset();
            let first = self.ident();
            let args = self.type_args();
            let mut segs = vec![(first, args, at)];
            while self.tok() == T::Dot {
                self.next();
                self.annotations();
                let seg_at = self.s.offset();
                let n = self.ident();
                let args = self.type_args();
                segs.push((n, args, seg_at));
            }
            Ty::Named(segs, at)
        } else {
            match self.tok() {
                T::Byte | T::Short | T::Char | T::Int | T::Long | T::Float | T::Double | T::Boolean => {
                    let t = self.tok();
                    self.next();
                    Ty::Prim(t)
                }
                _ => {
                    let at = self.s.offset();
                    self.err(at, "illegal start of type".to_string());
                    self.next();
                    Ty::Error
                }
            }
        };
        self.opt_array_brackets(t)
    }

    fn type_args(&mut self) -> Vec<Arg> {
        if self.tok() != T::Lt {
            return Vec::new();
        }
        self.next();
        let mut args = vec![self.type_arg()];
        while self.tok() == T::Comma {
            self.next();
            args.push(self.type_arg());
        }
        self.accept_closing_angle();
        args
    }

    fn type_arg(&mut self) -> Arg {
        if self.tok() == T::QMark {
            self.next();
            let hi = if self.tok() == T::Extends {
                self.next();
                Some(Box::new(self.typ()))
            } else {
                None
            };
            let lo = if self.tok() == T::Super {
                self.next();
                Some(Box::new(self.typ()))
            } else {
                None
            };
            Arg::Wild(hi, lo)
        } else {
            Arg::Ty(self.typ())
        }
    }

    fn annotations(&mut self) -> Vec<Annot> {
        let mut out = Vec::new();
        while self.tok() == T::At {
            self.next();
            if let Some(a) = self.annotation() {
                out.push(a);
            }
        }
        out
    }

    /// `annotation`: its arguments read; `None` where one cannot be (the annotation then dropped, silently).
    fn annotation(&mut self) -> Option<Annot> {
        let name = self.qual_id();
        let mut ok = true;
        if self.tok() == T::LParen {
            self.next();
            if self.tok() != T::RParen {
                ok &= self.ann_arg();
                while self.tok() == T::Comma {
                    self.next();
                    ok &= self.ann_arg();
                }
            }
            self.accept(T::RParen);
        }
        ok.then_some(Annot { name })
    }

    fn ann_arg(&mut self) -> bool {
        if self.tok() == T::Ident && self.s.lookahead() == T::Equals {
            self.ident();
            self.accept(T::Equals);
        }
        self.arg_value()
    }

    fn class_or_id(&mut self) {
        if self.tok() == T::Dot && self.s.lookahead() == T::Class {
            self.accept(T::Dot);
            self.accept(T::Class);
        }
    }

    fn arg_value(&mut self) -> bool {
        let ok = if self.try_constant().is_some() {
            true
        } else {
            match self.tok() {
                T::At => {
                    self.next();
                    self.annotation().is_some()
                }
                T::LBrace => {
                    self.accept(T::LBrace);
                    let mut all = true;
                    while self.tok() != T::RBrace && self.tok() != T::Eof {
                        let began = self.mark();
                        all &= self.arg_value();
                        if self.tok() == T::Comma {
                            self.next();
                        } else if self.mark() == began {
                            // an element that is none (`{)`): the array ends here, `accept` below says where
                            all = false;
                            break;
                        }
                    }
                    self.accept(T::RBrace);
                    all
                }
                T::Ident => {
                    self.qual_id();
                    self.class_or_id();
                    true
                }
                T::Byte | T::Short | T::Char | T::Int | T::Long | T::Float | T::Double | T::Boolean => {
                    self.next();
                    self.class_or_id();
                    true
                }
                _ => false,
            }
        };
        if matches!(self.tok(), T::Comma | T::RBrace | T::RParen) {
            ok
        } else {
            self.skip_to(&[T::Comma, T::RBrace, T::RParen]);
            false
        }
    }

    fn modifiers(&mut self, leading: Vec<Annot>) -> Mods {
        let mut m = Mods { flags: 0, annots: leading };
        loop {
            let f = match self.tok() {
                T::At if self.s.lookahead() != T::Interface => {
                    self.next();
                    if let Some(a) = self.annotation() {
                        m.annots.push(a);
                    }
                    continue;
                }
                T::Public => PUBLIC,
                T::Protected => PROTECTED,
                T::Private => PRIVATE,
                T::Static => STATIC,
                T::Abstract => ABSTRACT,
                T::Final => FINAL,
                T::Default => DEFAULT,
                T::Native => NATIVE,
                T::Transient => TRANSIENT,
                T::Volatile => VOLATILE,
                T::Synchronized => SYNCHRONIZED,
                T::Strictfp => 0,
                T::Sealed => SEALED,
                T::Ident if self.s.name() == "non" => {
                    let at = self.s.offset();
                    let name = self.s.name().to_string();
                    let sealed = self.s.lookahead() == T::Minus && self.s.lookahead2().1 == "sealed";
                    if sealed {
                        self.next();
                        self.next();
                        m.flags |= NON_SEALED;
                    } else {
                        self.err(at, format!("Identifier '{}' is not allowed here", name));
                    }
                    self.next();
                    continue;
                }
                _ => return m,
            };
            m.flags |= f;
            self.next();
        }
    }

    fn type_params(&mut self) -> Vec<TParam> {
        if self.tok() != T::Lt {
            return Vec::new();
        }
        self.next();
        let mut out = vec![self.type_param()];
        while self.tok() == T::Comma {
            self.next();
            out.push(self.type_param());
        }
        self.accept_closing_angle();
        out
    }

    fn type_param(&mut self) -> TParam {
        self.annotations();
        let at = self.s.offset();
        let name = self.ident();
        let mut bounds = Vec::new();
        if self.tok() == T::Extends {
            self.next();
            bounds.push(self.typ());
            while self.tok() == T::Amp {
                self.next();
                bounds.push(self.typ());
            }
        }
        TParam { name, at, bounds }
    }

    fn formal_params(&mut self) -> Vec<Param> {
        self.accept(T::LParen);
        let mut out = Vec::new();
        if self.tok() != T::RParen {
            out.push(self.formal_param());
            while self.tok() == T::Comma {
                self.next();
                out.push(self.formal_param());
            }
        }
        self.accept(T::RParen);
        out
    }

    fn formal_param(&mut self) -> Param {
        if self.tok() == T::Final {
            self.next();
        }
        self.annotations();
        let mut ty = self.typ();
        if self.tok() == T::DotDotDot {
            self.next();
            ty = Ty::Repeated(Box::new(ty));
        }
        let at = self.s.offset();
        let name = self.ident();
        let ty = self.opt_array_brackets(ty);
        Param { ty, name, at }
    }

    fn opt_throws(&mut self) {
        if self.tok() == T::Throws {
            self.next();
            self.typ();
            while self.tok() == T::Comma {
                self.next();
                self.typ();
            }
        }
    }

    fn method_body(&mut self) {
        self.skip_ahead();
        self.accept(T::RBrace);
    }

    /// `tryConstant`: one literal after an optional `-` or `!`.
    fn try_constant(&mut self) -> Option<Lit> {
        let negate = matches!(self.tok(), T::Minus | T::Bang);
        if negate {
            self.next();
        }
        let l = match self.tok() {
            T::True => Some(Lit::Bool(!negate)),
            T::False => Some(Lit::Bool(negate)),
            T::CharLit => Some(Lit::Char(self.s.str_val().chars().next().unwrap_or('\0'))),
            T::IntLit => self.s.int_val(negate).map(|v| Lit::Int(v as i32)),
            T::LongLit => self.s.int_val(negate).map(Lit::Long),
            T::FloatLit => self.s.float_val(negate).map(|v| Lit::Float(v as f32)),
            T::DoubleLit => self.s.float_val(negate).map(Lit::Double),
            T::StringLit => Some(Lit::Str(self.s.str_val().to_string())),
            _ => None,
        };
        // A malformed number is a literal all the same: the reference's `intVal` reports it and gives 0.
        let l = match (l, self.tok()) {
            (None, T::IntLit) => Some(Lit::Int(0)),
            (None, T::LongLit) => Some(Lit::Long(0)),
            (None, T::FloatLit) => Some(Lit::Float(0.0)),
            (None, T::DoubleLit) => Some(Lit::Double(0.0)),
            (l, _) => l,
        };
        if l.is_some() {
            self.next();
        }
        l
    }

    /// `varDecl`: a field's array brackets after its name, and its constant: a static final field's initializer
    /// that is one literal followed by `;` (`optConstantTpe`). The initializer skipped.
    fn var_decl(&mut self, mods: &Mods, ty: Ty) -> (Ty, Option<Lit>) {
        let ty = self.opt_array_brackets(ty);
        let mut constant = None;
        if self.tok() == T::Equals {
            self.next();
            if mods.flags & STATIC != 0 && mods.flags & FINAL != 0 {
                if let Some(l) = self.try_constant() {
                    if self.tok() == T::Semi {
                        constant = Some(l);
                    }
                }
            }
            self.skip_to(&[T::Comma, T::Semi]);
        }
        (ty, constant)
    }

    fn field_decls(&mut self, mods: &Mods, ty: Ty, name: String, at: u32, out: &mut Vec<Member>) {
        let (t, c) = self.var_decl(mods, ty.clone());
        out.push(Member::Field { mods: mods.clone(), ty: t, name, constant: c, at });
        let mut maybe: Vec<Member> = Vec::new();
        while self.tok() == T::Comma {
            self.next();
            if self.tok() == T::Ident {
                let at = self.s.offset();
                let n = self.ident();
                if matches!(self.tok(), T::Equals | T::Semi) {
                    out.append(&mut maybe);
                    let (t, c) = self.var_decl(mods, ty.clone());
                    out.push(Member::Field { mods: mods.clone(), ty: t, name: n, constant: c, at });
                } else if self.tok() == T::Comma {
                    let (t, c) = self.var_decl(mods, ty.clone());
                    maybe.push(Member::Field { mods: mods.clone(), ty: t, name: n, constant: c, at });
                } else {
                    self.skip_to(&[T::Comma, T::Semi]);
                    maybe.clear();
                }
            } else {
                self.skip_to(&[T::Comma, T::Semi]);
                maybe.clear();
            }
        }
        if self.tok() == T::Semi {
            out.append(&mut maybe);
        }
    }

    fn is_type_decl_start(&mut self) -> bool {
        self.adapt_record();
        matches!(self.tok(), T::Class | T::Enum | T::Interface | T::At) || self.record
    }

    fn adapt_record(&mut self) {
        self.record = self.tok() == T::Ident && self.s.name() == "record";
    }

    fn term_decl(&mut self, start: u32, mods: Mods, parent: Kind, parent_name: &str, out: &mut Vec<Member>) {
        let _ = start;
        let in_interface = matches!(parent, Kind::Interface | Kind::Annotation);
        let tparams = if self.tok() == T::Lt { self.type_params() } else { Vec::new() };
        let is_void = self.tok() == T::Void;
        let ret = if is_void {
            self.next();
            Ty::Void
        } else {
            self.typ()
        };
        let simple = matches!(&ret, Ty::Named(segs, _) if segs.len() == 1 && segs[0].1.is_empty());
        if self.tok() == T::LParen && simple && !in_interface {
            let params = self.formal_params();
            self.opt_throws();
            self.method_body();
            out.push(Member::Ctor { mods, tparams, params });
            return;
        }
        if self.tok() == T::LBrace && simple && parent == Kind::Record {
            // a record's compact constructor
            self.method_body();
            let _ = parent_name;
            return;
        }
        let at = self.s.offset();
        let name = self.ident();
        if self.tok() == T::LParen {
            let params = self.formal_params();
            let ret = if is_void { ret } else { self.opt_array_brackets(ret) };
            self.opt_throws();
            let body_ok = !in_interface || mods.flags & (DEFAULT | STATIC | PRIVATE) != 0;
            let mut annotation_default = false;
            let body = if body_ok && self.tok() == T::LBrace {
                self.method_body();
                true
            } else if parent == Kind::Annotation && self.tok() == T::Default {
                annotation_default = true;
                self.skip_to(&[T::Semi]);
                self.accept(T::Semi);
                false
            } else {
                self.accept(T::Semi);
                false
            };
            out.push(Member::Method { mods, tparams, ret, name, params, body, annotation_default, at });
        } else {
            let mut mods = mods;
            if in_interface {
                mods.flags |= FINAL | STATIC;
            }
            self.field_decls(&mods, ret, name, at, out);
            self.accept(T::Semi);
        }
    }

    fn type_body(&mut self, kind: Kind, name: &str) -> Vec<Member> {
        self.accept(T::LBrace);
        let m = self.type_body_decls(kind, name);
        self.accept(T::RBrace);
        m
    }

    fn type_body_decls(&mut self, kind: Kind, name: &str) -> Vec<Member> {
        let in_interface = matches!(kind, Kind::Interface | Kind::Annotation);
        let mut out = Vec::new();
        while self.tok() != T::RBrace && self.tok() != T::Eof {
            let began = self.mark();
            let start = self.s.offset();
            let mut mods = self.modifiers(Vec::new());
            if self.tok() == T::LBrace {
                self.skip_ahead();
                self.accept(T::RBrace);
            } else if self.tok() == T::Semi {
                self.next();
            } else if self.is_type_decl_start() {
                if in_interface || matches!(self.tok(), T::Enum | T::Interface | T::At) || self.record {
                    mods.flags |= STATIC;
                }
                if let Some(t) = self.type_decl(start, mods) {
                    out.push(Member::Type(t));
                }
            } else {
                self.term_decl(start, mods, kind, name, &mut out);
            }
            self.progress(began, "declaration");
        }
        out
    }

    fn interfaces_opt(&mut self) -> Vec<Ty> {
        let mut out = Vec::new();
        if self.tok() == T::Implements {
            self.next();
            out.push(self.typ());
            while self.tok() == T::Comma {
                self.next();
                out.push(self.typ());
            }
        }
        out
    }

    fn permits_opt(&mut self, sealed: bool) -> Vec<Ty> {
        let mut out = Vec::new();
        if self.tok() == T::Permits && !sealed {
            let at = self.s.offset();
            self.err(at, "A type declaration that has a permits clause should have a sealed modifier".to_string());
        }
        if self.tok() == T::Permits {
            self.next();
            out.push(self.typ());
            while self.tok() == T::Comma {
                self.next();
                out.push(self.typ());
            }
        }
        out
    }

    fn decl(&self, kind: Kind, name: String, start: u32, offset: u32, mods: Mods) -> TypeDecl {
        TypeDecl { kind, name, start, offset, mods, tparams: Vec::new(), superclass: None, interfaces: Vec::new(), permits: Vec::new(), members: Vec::new(), constants: Vec::new(), header: Vec::new() }
    }

    fn type_decl(&mut self, start: u32, mods: Mods) -> Option<TypeDecl> {
        if self.record {
            self.record = false;
            self.next();
            let offset = self.s.offset();
            let name = self.ident();
            let mut d = self.decl(Kind::Record, name.clone(), start, offset, mods);
            d.tparams = self.type_params();
            d.header = self.formal_params();
            d.interfaces = self.interfaces_opt();
            d.members = self.type_body(Kind::Record, &name);
            return Some(d);
        }
        match self.tok() {
            T::Class => {
                self.next();
                let offset = self.s.offset();
                let name = self.ident();
                let mut d = self.decl(Kind::Class, name.clone(), start, offset, mods);
                d.tparams = self.type_params();
                if self.tok() == T::Extends {
                    self.next();
                    d.superclass = Some(self.typ());
                }
                d.interfaces = self.interfaces_opt();
                d.permits = self.permits_opt(d.mods.flags & SEALED != 0);
                d.members = self.type_body(Kind::Class, &name);
                Some(d)
            }
            T::Interface => {
                self.next();
                let offset = self.s.offset();
                let name = self.ident();
                let mut d = self.decl(Kind::Interface, name.clone(), start, offset, mods);
                d.tparams = self.type_params();
                if self.tok() == T::Extends {
                    self.next();
                    d.interfaces.push(self.typ());
                    while self.tok() == T::Comma {
                        self.next();
                        d.interfaces.push(self.typ());
                    }
                }
                d.permits = self.permits_opt(d.mods.flags & SEALED != 0);
                d.members = self.type_body(Kind::Interface, &name);
                Some(d)
            }
            T::At => {
                self.accept(T::At);
                self.accept(T::Interface);
                let offset = self.s.offset();
                let name = self.ident();
                let mut d = self.decl(Kind::Annotation, name.clone(), start, offset, mods);
                d.members = self.type_body(Kind::Annotation, &name);
                Some(d)
            }
            T::Enum => {
                self.next();
                let offset = self.s.offset();
                let name = self.ident();
                let mut d = self.decl(Kind::Enum, name.clone(), start, offset, mods);
                d.interfaces = self.interfaces_opt();
                self.accept(T::LBrace);
                while !matches!(self.tok(), T::RBrace | T::Semi | T::Eof) {
                    self.annotations();
                    let at = self.s.offset();
                    let c = self.ident();
                    if self.tok() == T::LParen {
                        self.skip_ahead();
                        self.accept(T::RParen);
                    }
                    if self.tok() == T::LBrace {
                        self.skip_ahead();
                        self.accept(T::RBrace);
                    }
                    d.constants.push((c, at));
                    if self.tok() == T::Comma {
                        self.next();
                    } else {
                        break;
                    }
                }
                if self.tok() == T::Semi {
                    self.next();
                    d.members = self.type_body_decls(Kind::Enum, &name);
                }
                self.accept(T::RBrace);
                Some(d)
            }
            _ => {
                // `typeDecl`'s last case: past the token, "illegal start of type declaration" where the parser then
                // stands, and the rest skipped to a `;` or a `}` (`syntaxError(.., skipIt = true)`).
                self.next();
                let at = self.s.offset();
                self.err(at, "illegal start of type declaration".to_string());
                self.skip_to(&[T::Semi, T::RBrace]);
                None
            }
        }
    }

    fn import_decl(&mut self) -> Option<Import> {
        let offset = self.accept(T::Import);
        let is_static = self.tok() == T::Static;
        if is_static {
            self.next();
        }
        let mut names = Vec::new();
        let mut at = Vec::new();
        loop {
            at.push(self.s.offset());
            if self.tok() == T::Asterisk {
                self.next();
                names.push("*".to_string());
                break;
            }
            names.push(self.ident());
            if self.tok() == T::Dot {
                self.next();
            } else {
                break;
            }
        }
        self.accept(T::Semi);
        // `import a;`: the reference counts the root package's name in
        if names.len() + usize::from(!is_static) < 2 {
            self.err(offset, "illegal import".to_string());
            return None;
        }
        Some(Import { is_static, names, at })
    }
}

impl P {
    fn unit(&mut self) -> Unit {
        let mut u = Unit::default();
        let leading = if self.tok() == T::At { self.annotations() } else { Vec::new() };
        let mut imports = true;
        if self.tok() == T::Package {
            self.accept(T::Package);
            u.package = self.qual_id();
            self.accept(T::Semi);
        } else if !leading.is_empty() {
            // `compilationUnit`: annotations before anything but a package are a type declaration's, whatever follows
            // them ("illegal start of type declaration" where none starts), and no import is read after it. A record
            // is one, as javac reads it, where the reference does not adapt the identifier there (a Java source's
            // validity is javac's: docs/COMPATIBILITY.md).
            let start = self.s.offset();
            let mods = self.modifiers(leading);
            self.adapt_record();
            if let Some(t) = self.type_decl(start, mods) {
                u.types.push(t);
            }
            imports = false;
        }
        if imports {
            while self.tok() == T::Import {
                if let Some(i) = self.import_decl() {
                    u.imports.push(i);
                }
            }
        }
        while self.tok() != T::Eof && self.tok() != T::RBrace {
            let began = self.mark();
            while self.tok() == T::Semi {
                self.next();
            }
            if self.tok() != T::Eof {
                let start = self.s.offset();
                let mods = self.modifiers(Vec::new());
                if self.is_type_decl_start() {
                    if let Some(t) = self.type_decl(start, mods) {
                        u.types.push(t);
                    }
                } else {
                    let mut members = Vec::new();
                    self.term_decl(start, mods, Kind::Class, "", &mut members);
                    if !members.is_empty() && u.compact.is_none() {
                        u.compact = Some(start);
                    }
                }
            }
            self.progress(began, "type declaration");
        }
        self.accept(T::Eof);
        u
    }
}

/// Reads a Java source's declarations.
pub fn parse(text: &str) -> Unit {
    let mut p = P { s: Scanner::new(text), errors: Vec::new(), record: false };
    let mut u = p.unit();
    let mut errors = std::mem::take(&mut p.s.errors);
    errors.append(&mut p.errors);
    errors.sort_by_key(|e| e.0);
    u.errors = errors;
    u
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_sealed_is_read_by_its_context() {
        let u = parse("package p;\npublic sealed class S permits A {}\nfinal class B {}\npublic non-sealed class A extends S {}\n");
        assert!(u.errors.is_empty(), "{:?}", u.errors);
        let a = u.types.iter().find(|t| t.name == "A").unwrap();
        assert!(a.mods.flags & NON_SEALED != 0);
        // `non` that no `-sealed` follows is the reference's error at the name
        let u = parse("package p;\nclass C { non int x; }\n");
        assert_eq!(u.errors.iter().map(|e| e.1.as_str()).collect::<Vec<_>>(), vec!["Identifier 'non' is not allowed here"]);
    }

    #[test]
    fn an_array_element_that_is_none_ends_the_array() {
        // `{)`: the element reads nothing; the array ends at the `)`, told there, and the class is read on
        let text = "@SuppressWarnings({) class J { int x; }\n";
        let u = parse(text);
        assert_eq!(u.errors, vec![(text.find(')').unwrap() as u32, "'}' expected but ')' found.".to_string())]);
        assert!(u.types.iter().any(|t| t.name == "J"));
    }

    #[test]
    fn a_round_that_reads_nothing_is_past_its_token() {
        // a token no member can start, in a class's body and at the top: an error there, and the rest read
        for text in ["class J { ) int x; }\n", ") class J { }\n", "class J { ]] }\n"] {
            let u = parse(text);
            let first = text.find([')', ']']).unwrap() as u32;
            assert_eq!(u.errors.first().map(|e| e.0), Some(first), "{text}: {:?}", u.errors);
            assert!(u.errors[0].1.starts_with("illegal start of"), "{text}: {:?}", u.errors);
            assert!(u.types.iter().any(|t| t.name == "J"), "{text}");
        }
    }

    #[test]
    fn an_identifier_goes_on_as_the_reference_reads_it() {
        // a combining mark (U+0301) goes on an identifier; a code point above U+FFFF is a surrogate pair to the
        // reference, an illegal character a UTF-16 unit, and the scanner is past it
        let u = parse("class J { int a\u{301}() { return 7; } }\n");
        assert!(u.errors.is_empty(), "{:?}", u.errors);
        let u = parse("class J { int a\u{10400}b; }\n");
        let at = "class J { int a".len() as u32;
        assert_eq!(u.errors.iter().take(2).map(|e| (e.0, e.1.as_str())).collect::<Vec<_>>(),
                   vec![(at, "illegal character: 55297"), (at, "illegal character: 56320")], "{:?}", u.errors);
    }

    #[test]
    fn annotations_before_no_type_declaration_are_refused() {
        // annotations at the top, before anything but a package, are a type declaration's: where none starts, the
        // error stands on the token after the one that starts none (scalac's places: 3, 10, 4 and 19)
        for (text, at) in [("@Su", 3), ("@Su public", 10), ("@Su;", 4), ("@Deprecated import java.util.List; class J {}", 19)] {
            let u = parse(text);
            assert_eq!(u.errors.first().map(|e| (e.0, e.1.as_str())), Some((at, "illegal start of type declaration")), "{text}: {:?}", u.errors);
        }
        let u = parse("@Deprecated public class J {}");
        assert!(u.errors.is_empty() && u.types.iter().any(|t| t.name == "J"), "{:?}", u.errors);
        // a record so annotated is read, as javac reads it
        for text in ["@Deprecated public record R(int x) {}", "@Deprecated record R(int x) {}", "@SuppressWarnings(\"x\")\npublic record R(int x) {}"] {
            let u = parse(text);
            assert!(u.errors.is_empty() && u.types.iter().any(|t| t.name == "R" && t.kind == Kind::Record), "{text}: {:?}", u.errors);
        }
    }

    #[test]
    fn an_illegal_character_is_one_character() {
        // the scanner is past an illegal character and no further: the `}` after it closes the class
        let u = parse("class J { #}\n");
        assert_eq!(u.errors.first().map(|e| (e.0, e.1.as_str())), Some((10, "illegal character: 35")), "{:?}", u.errors);
        assert!(!u.errors.iter().any(|e| e.1.contains("eof")), "{:?}", u.errors);
        assert!(u.types.iter().any(|t| t.name == "J"));
    }

    #[test]
    fn a_compact_source_is_placed_at_its_first_member() {
        let text = "import java.io.File;\n\nint k = 0;\nvoid main() { }\nclass H { }\n";
        let u = parse(text);
        assert_eq!(u.compact, Some(text.find("int k").unwrap() as u32));
        assert!(parse("class H { int k; }\n").compact.is_none());
    }
}
