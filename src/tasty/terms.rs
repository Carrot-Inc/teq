//! The term grammar: bodies of definitions, decoded on demand from the address a signature
//! keeps (`DefSig::body`) or from a class's template. Types inside terms go through the type
//! decoder of `tree.rs`; a tag without a reading becomes `Term::Unknown` and is skipped, so that
//! the cursor moves on either way.

use super::tags::*;
use super::tree::*;
use super::NameRef;

#[derive(Clone, Copy, Debug)]
pub struct Span {
    pub start: Addr,
    pub end: Addr,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MatchKind {
    Plain,
    Inline,
    /// `inline x match` without a selector: `summonFrom`.
    Implicit,
    Sub,
}

#[derive(Clone, Debug)]
pub struct Case {
    pub pat: Term,
    pub guard: Option<Term>,
    pub body: Term,
}

/// A tree of a body with the address it was read at: for a shared reference the address of the
/// occurrence, whose position is its own where the pickle gives it one, the
/// tree under it read at the target's.
#[derive(Clone, Debug)]
pub struct Term {
    pub at: Addr,
    pub kind: TermKind,
}

impl Term {
    pub fn new(at: Addr, kind: TermKind) -> Term {
        Term { at, kind }
    }
}

#[derive(Clone, Debug)]
pub enum TermKind {
    /// A reference by symbol or by name, `this`, or a type tree in a term position.
    Path(TType),
    Const(Const),
    Ident(NameRef, TType),
    Select(Box<Term>, NameRef),
    /// `qual.name` resolved in the class `owner` declares it, with the signature in the name, and
    /// the address the owner's type is pickled at (a shared type's target), which names it.
    SelectIn(Box<Term>, NameRef, TType, Addr),
    QualThis(TType),
    New(TType),
    Throw(Box<Term>),
    NamedArg(NameRef, Box<Term>),
    Apply(Box<Term>, Vec<Term>),
    ApplySigPoly(Box<Term>, Box<TType>, Vec<Term>),
    /// With whether scalac inferred the type arguments.
    TypeApply(Box<Term>, Vec<TType>, bool),
    /// `this.super[mixin]`
    Super(Box<Term>, Option<TType>),
    /// With whether scalac inserted the ascription's type.
    Typed(Box<Term>, TType, bool),
    Assign(Box<Term>, Box<Term>),
    Block(Vec<Stat>, Box<Term>),
    If { inline: bool, cond: Box<Term>, then: Box<Term>, els: Box<Term> },
    /// A closure over the local method `meth`; the type is given where it is not a function type.
    Lambda(Box<Term>, Option<TType>),
    Match { kind: MatchKind, selector: Option<Box<Term>>, cases: Vec<Case> },
    Try { body: Box<Term>, cases: Vec<Case>, finalizer: Option<Box<Term>> },
    /// `return expr` from the method whose `DEFDEF` is at `from`.
    Return { from: Addr, expr: Option<Box<Term>> },
    While(Box<Term>, Box<Term>),
    /// The expansion of an inline call, with the call it came from and its address (absent
    /// when inlined from an outer scope) and the bindings of its arguments.
    Inlined { expansion: Box<Term>, call: Option<Box<Term>>, call_at: Option<Addr>, bindings: Vec<Stat> },
    /// A vararg argument: the element type and the elements.
    Repeated(Box<TType>, Vec<Term>),
    /// `levels` outer links followed from `qual`.
    SelectOuter { levels: u32, qual: Box<Term>, ty: TType },
    Quote { body: Box<Term>, ty: TType, span: Span },
    Splice { expr: Box<Term>, ty: TType },
    QuotePattern { body: Box<Term>, quotes: Box<Term>, ty: Box<TType>, bindings: Vec<Stat>, span: Span },
    SplicePattern { pat: Box<Term>, ty: Box<TType>, targs: Vec<TType>, args: Vec<Term> },
    /// `name @ body`; `addr` is what a `TERMREFdirect` to the bound variable names.
    Bind { addr: Addr, name: NameRef, ty: Box<TType>, body: Box<Term>, flags: Flags },
    Alternative(Vec<Term>),
    Unapply { fun: Box<Term>, implicits: Vec<Term>, ty: Box<TType>, pats: Vec<Term> },
    Elided(TType),
    Hole { idx: u32, ty: Box<TType>, args: Vec<Term> },
    /// A tag this reader has no reading for in a term position, at its address.
    Unknown(u8, Addr),
    /// A tree past the depth limit, skipped.
    TooDeep(Addr),
    /// A shared reference to a tree being decoded.
    Cycle(Addr),
}

#[derive(Clone, Debug)]
pub enum Stat {
    Val(DefSig, Option<Term>),
    Def(DefSig, Option<Term>),
    Type(TypeDefSig),
    Class(Box<ClassDef>),
    Import { path: TType, selectors: Vec<(NameRef, Option<NameRef>)> },
    Expr(Term),
}

#[derive(Clone, Debug)]
pub struct Template {
    pub tparams: Vec<TParam>,
    pub params: Vec<Param>,
    pub parents: Vec<Term>,
    pub self_type: Option<(NameRef, TType)>,
    pub ctor: Option<(DefSig, Option<Term>)>,
    pub stats: Vec<Stat>,
}

#[derive(Clone, Debug)]
pub struct ClassDef {
    /// What a `TYPEREFdirect` to the class names.
    pub addr: Addr,
    pub name: NameRef,
    pub mods: Mods,
    pub template: Template,
}

/// Terms nest deeper than types (a chain of `+` is left-deep); the guard skips what is deeper.
const MAX_TERM_DEPTH: u32 = 512;

pub struct TermDecoder<'a, 'd> {
    pub d: &'d mut Decoder<'a>,
    depth: u32,
    pub max_depth: u32,
}

impl<'a, 'd> TermDecoder<'a, 'd> {
    pub fn new(d: &'d mut Decoder<'a>) -> TermDecoder<'a, 'd> {
        TermDecoder { d, depth: 0, max_depth: 0 }
    }

    fn pos(&self) -> usize {
        self.d.r.pos
    }

    fn peek_unshared(&self) -> u8 {
        let mut r = self.d.r.clone();
        if r.peek() == SHAREDTERM {
            r.byte();
            let addr = r.nat() as usize;
            r = r.at(addr);
        }
        r.peek()
    }

    fn typedef_is_class(&self, addr: Addr) -> bool {
        let mut r = self.d.r.at(addr as usize);
        r.byte();
        r.end();
        r.nat();
        r.peek() == TEMPLATE
    }

    /// The term at `addr`; the cursor is left behind it.
    pub fn term_at(&mut self, addr: Addr) -> Term {
        self.d.r.pos = addr as usize;
        self.read_term()
    }

    /// The statement at `addr`: a definition with its body, or a term.
    pub fn stat_at(&mut self, addr: Addr) -> Stat {
        self.d.r.pos = addr as usize;
        self.stat()
    }

    fn nested(&mut self, f: impl FnOnce(&mut Self) -> Term) -> Term {
        if self.depth >= MAX_TERM_DEPTH {
            let at = self.pos() as Addr;
            self.d.skip_tree();
            return Term::new(at, TermKind::TooDeep(at));
        }
        self.depth += 1;
        self.max_depth = self.max_depth.max(self.depth);
        let t = f(self);
        self.depth -= 1;
        t
    }

    fn shared<T>(&mut self, addr: Addr, cycle: T, f: impl FnOnce(&mut Self) -> T) -> T {
        if self.d.open_shared.contains(&addr) {
            return cycle;
        }
        let saved = self.pos();
        self.d.r.pos = addr as usize;
        self.d.open_shared.push(addr);
        let t = f(self);
        self.d.open_shared.pop();
        self.d.r.pos = saved;
        t
    }

    pub fn read_term(&mut self) -> Term {
        self.nested(|t| t.read_term_now())
    }

    fn boxed(&mut self) -> Box<Term> {
        Box::new(self.read_term())
    }

    fn terms_until(&mut self, end: usize) -> Vec<Term> {
        let mut out = Vec::new();
        while self.pos() < end {
            out.push(self.read_term());
        }
        out
    }

    fn read_term_now(&mut self) -> Term {
        let start = self.pos() as Addr;
        if self.d.r.peek() == SHAREDTERM {
            self.d.r.byte();
            let addr = self.d.r.nat();
            let target = self.shared(addr, Term::new(start, TermKind::Cycle(addr)), |t| t.read_term_now());
            return Term::new(start, target.kind);
        }
        let kind = self.read_kind(start);
        Term::new(start, kind)
    }

    fn read_kind(&mut self, start: Addr) -> TermKind {
        let tag = self.d.r.peek();
        match tag {
            IDENT => {
                self.d.r.byte();
                let name = self.d.r.nat();
                TermKind::Ident(name, self.d.read_type())
            }
            SELECT => {
                self.d.r.byte();
                let name = self.d.r.nat();
                TermKind::Select(self.boxed(), name)
            }
            SELECTIN => {
                self.d.r.byte();
                let end = self.d.r.end();
                let name = self.d.r.nat();
                let qual = self.boxed();
                let owner_at = self.d.shared_target(self.pos() as Addr);
                let owner = self.d.read_type();
                self.d.r.pos = end;
                TermKind::SelectIn(qual, name, owner, owner_at)
            }
            QUALTHIS => {
                self.d.r.byte();
                TermKind::QualThis(self.d.read_tpt())
            }
            NEW => {
                self.d.r.byte();
                TermKind::New(self.d.read_tpt())
            }
            THROW => {
                self.d.r.byte();
                TermKind::Throw(self.boxed())
            }
            ELIDED => {
                self.d.r.byte();
                TermKind::Elided(self.d.read_type())
            }
            EXPLICITTPT => {
                self.d.r.byte();
                TermKind::Path(self.d.read_tpt())
            }
            NAMEDARG => {
                self.d.r.byte();
                let name = self.d.r.nat();
                TermKind::NamedArg(name, self.boxed())
            }
            APPLY => {
                self.d.r.byte();
                let end = self.d.r.end();
                let f = self.boxed();
                let args = self.terms_until(end);
                TermKind::Apply(f, args)
            }
            APPLYSIGPOLY => {
                self.d.r.byte();
                let end = self.d.r.end();
                let f = self.boxed();
                let ty = self.d.read_type();
                let args = self.terms_until(end);
                TermKind::ApplySigPoly(f, Box::new(ty), args)
            }
            TYPEAPPLY => {
                self.d.r.byte();
                let end = self.d.r.end();
                let f = self.boxed();
                let mut targs = Vec::new();
                let inferred = self.pos() < end && self.d.tpt_is_inferred();
                while self.pos() < end {
                    targs.push(self.d.read_tpt());
                }
                TermKind::TypeApply(f, targs, inferred)
            }
            SUPER => {
                self.d.r.byte();
                let end = self.d.r.end();
                let this = self.boxed();
                let mixin = (self.pos() < end).then(|| self.d.read_tpt());
                self.d.r.pos = end;
                TermKind::Super(this, mixin)
            }
            TYPED => {
                self.d.r.byte();
                let end = self.d.r.end();
                let expr = self.boxed();
                let inferred = self.d.tpt_is_inferred();
                let ty = self.d.read_tpt();
                self.d.r.pos = end;
                TermKind::Typed(expr, ty, inferred)
            }
            ASSIGN => {
                self.d.r.byte();
                let end = self.d.r.end();
                let lhs = self.boxed();
                let rhs = self.boxed();
                self.d.r.pos = end;
                TermKind::Assign(lhs, rhs)
            }
            BLOCK => {
                self.d.r.byte();
                let end = self.d.r.end();
                let expr = self.boxed();
                let stats = self.stats(end);
                TermKind::Block(stats, expr)
            }
            INLINED => {
                self.d.r.byte();
                let end = self.d.r.end();
                let expansion = self.boxed();
                let call_at = (self.pos() < end && !matches!(self.peek_unshared(), VALDEF | DEFDEF)).then(|| self.pos() as Addr);
                let call = call_at.map(|_| self.boxed());
                let bindings = self.stats(end);
                TermKind::Inlined { expansion, call, call_at, bindings }
            }
            IF => {
                self.d.r.byte();
                let end = self.d.r.end();
                let inline = self.d.r.peek() == INLINE;
                if inline {
                    self.d.r.byte();
                }
                let cond = self.boxed();
                let then = self.boxed();
                let els = self.boxed();
                self.d.r.pos = end;
                TermKind::If { inline, cond, then, els }
            }
            LAMBDA => {
                self.d.r.byte();
                let end = self.d.r.end();
                let meth = self.boxed();
                let ty = (self.pos() < end).then(|| self.d.read_tpt());
                self.d.r.pos = end;
                TermKind::Lambda(meth, ty)
            }
            MATCH => {
                self.d.r.byte();
                let end = self.d.r.end();
                let (kind, selector) = match self.d.r.peek() {
                    IMPLICIT => {
                        self.d.r.byte();
                        (MatchKind::Implicit, None)
                    }
                    INLINE => {
                        self.d.r.byte();
                        (MatchKind::Inline, Some(self.boxed()))
                    }
                    SUBMATCH => {
                        self.d.r.byte();
                        (MatchKind::Sub, Some(self.boxed()))
                    }
                    _ => (MatchKind::Plain, Some(self.boxed())),
                };
                let cases = self.cases(end);
                self.d.r.pos = end;
                TermKind::Match { kind, selector, cases }
            }
            TRY => {
                self.d.r.byte();
                let end = self.d.r.end();
                let body = self.boxed();
                let cases = self.cases(end);
                let finalizer = (self.pos() < end).then(|| self.boxed());
                self.d.r.pos = end;
                TermKind::Try { body, cases, finalizer }
            }
            RETURN => {
                self.d.r.byte();
                let end = self.d.r.end();
                let from = self.d.r.nat();
                let expr = (self.pos() < end).then(|| self.boxed());
                self.d.r.pos = end;
                TermKind::Return { from, expr }
            }
            WHILE => {
                self.d.r.byte();
                let end = self.d.r.end();
                let cond = self.boxed();
                let body = self.boxed();
                self.d.r.pos = end;
                TermKind::While(cond, body)
            }
            SELECTOUTER => {
                self.d.r.byte();
                let end = self.d.r.end();
                let levels = self.d.r.nat();
                let qual = self.boxed();
                let ty = self.d.read_type();
                self.d.r.pos = end;
                TermKind::SelectOuter { levels, qual, ty }
            }
            REPEATED => {
                self.d.r.byte();
                let end = self.d.r.end();
                let elem = self.d.read_tpt();
                let elems = self.terms_until(end);
                TermKind::Repeated(Box::new(elem), elems)
            }
            QUOTE => {
                self.d.r.byte();
                let end = self.d.r.end();
                let body = self.boxed();
                let ty = self.d.read_type();
                self.d.r.pos = end;
                TermKind::Quote { body, ty, span: Span { start, end: end as Addr } }
            }
            SPLICE => {
                self.d.r.byte();
                let end = self.d.r.end();
                let expr = self.boxed();
                let ty = self.d.read_type();
                self.d.r.pos = end;
                TermKind::Splice { expr, ty }
            }
            QUOTEPATTERN => {
                self.d.r.byte();
                let end = self.d.r.end();
                let body = self.boxed();
                let quotes = self.boxed();
                let ty = self.d.read_type();
                let bindings = self.stats(end);
                TermKind::QuotePattern { body, quotes, ty: Box::new(ty), bindings, span: Span { start, end: end as Addr } }
            }
            SPLICEPATTERN => {
                self.d.r.byte();
                let end = self.d.r.end();
                let pat = self.boxed();
                let ty = self.d.read_type();
                let mut targs = Vec::new();
                let mut args = Vec::new();
                // The type arguments come first, each behind `EXPLICITtpt`, then the terms.
                while self.pos() < end {
                    if self.d.r.peek() == EXPLICITTPT {
                        self.d.r.byte();
                        targs.push(self.d.read_tpt());
                    } else {
                        args.push(self.read_term());
                    }
                }
                TermKind::SplicePattern { pat, ty: Box::new(ty), targs, args }
            }
            BIND => {
                self.d.r.byte();
                let end = self.d.r.end();
                let name = self.d.r.nat();
                let ty = self.d.read_type();
                let body = self.boxed();
                let flags = self.d.modifiers(end).flags;
                TermKind::Bind { addr: start, name, ty: Box::new(ty), body, flags }
            }
            ALTERNATIVE => {
                self.d.r.byte();
                let end = self.d.r.end();
                TermKind::Alternative(self.terms_until(end))
            }
            UNAPPLY => {
                self.d.r.byte();
                let end = self.d.r.end();
                let fun = self.boxed();
                let mut implicits = Vec::new();
                while self.d.r.peek() == IMPLICITARG && self.pos() < end {
                    self.d.r.byte();
                    implicits.push(self.read_term());
                }
                let ty = self.d.read_type();
                let pats = self.terms_until(end);
                TermKind::Unapply { fun, implicits, ty: Box::new(ty), pats }
            }
            HOLE => {
                self.d.r.byte();
                let end = self.d.r.end();
                let idx = self.d.r.nat();
                let ty = self.d.read_type();
                let args = self.terms_until(end);
                TermKind::Hole { idx, ty: Box::new(ty), args }
            }
            IDENTTPT | SELECTTPT | SINGLETONTPT | BYNAMETPT | APPLIEDTPT | LAMBDATPT | TYPEBOUNDSTPT | ANNOTATEDTPT
            | REFINEDTPT | MATCHTPT => TermKind::Path(self.d.read_tpt()),
            VALDEF | DEFDEF | TYPEDEF | IMPORT | EXPORT | PACKAGE | TEMPLATE | TYPEPARAM | PARAM | CASEDEF => {
                self.d.skip_tree();
                TermKind::Unknown(tag, start)
            }
            _ => match self.d.read_type() {
                TType::Const(c) => TermKind::Const(c),
                TType::Unknown(tag) => TermKind::Unknown(tag, start),
                t => TermKind::Path(t),
            },
        }
    }

    fn cases(&mut self, end: usize) -> Vec<Case> {
        let mut out = Vec::new();
        while self.pos() < end && self.peek_unshared() == CASEDEF {
            if self.d.r.peek() == SHAREDTERM {
                self.d.r.byte();
                let addr = self.d.r.nat();
                let cycle = Case { pat: Term::new(addr, TermKind::Cycle(addr)), guard: None, body: Term::new(addr, TermKind::Cycle(addr)) };
                let c = self.shared(addr, cycle, |t| t.case());
                out.push(c);
            } else {
                out.push(self.case());
            }
        }
        out
    }

    fn case(&mut self) -> Case {
        self.d.r.byte();
        let end = self.d.r.end();
        let pat = self.read_term();
        let body = self.read_term();
        let guard = (self.pos() < end).then(|| self.read_term());
        self.d.r.pos = end;
        Case { pat, guard, body }
    }

    fn stats(&mut self, end: usize) -> Vec<Stat> {
        let mut out = Vec::new();
        while self.pos() < end {
            out.push(self.stat());
        }
        out
    }

    fn stat(&mut self) -> Stat {
        let addr = self.pos() as Addr;
        match self.d.r.peek() {
            VALDEF | DEFDEF => {
                let (sig, rhs) = self.def_with_body(addr);
                if sig.tag == VALDEF {
                    Stat::Val(sig, rhs)
                } else {
                    Stat::Def(sig, rhs)
                }
            }
            TYPEDEF if self.typedef_is_class(addr) => Stat::Class(Box::new(self.class_def(addr))),
            TYPEDEF => Stat::Type(self.d.type_def_sig(addr)),
            IMPORT | EXPORT => {
                self.d.r.byte();
                let end = self.d.r.end();
                let e = self.d.export_sig(addr);
                self.d.r.pos = end;
                Stat::Import { path: e.path, selectors: e.selectors }
            }
            _ => Stat::Expr(self.read_term()),
        }
    }

    /// The signature of the definition at `addr` and its body; the cursor is left behind the
    /// definition.
    pub fn def_with_body(&mut self, addr: Addr) -> (DefSig, Option<Term>) {
        let sig = self.d.def_sig(addr);
        let after = self.pos();
        let rhs = sig.body.map(|b| {
            self.d.r.pos = b as usize;
            self.read_term()
        });
        self.d.r.pos = after;
        (sig, rhs)
    }

    /// The class at `addr` with its template decoded in full: parents as constructor calls,
    /// the constructor and every member with its body, the statements between them.
    pub fn class_def(&mut self, addr: Addr) -> ClassDef {
        self.d.r.pos = addr as usize;
        self.d.r.byte();
        let end = self.d.r.end();
        let name = self.d.r.nat();
        let mut template =
            Template { tparams: Vec::new(), params: Vec::new(), parents: Vec::new(), self_type: None, ctor: None, stats: Vec::new() };
        if self.d.r.peek() == TEMPLATE {
            self.d.r.byte();
            let template_end = self.d.r.end();
            while self.d.r.peek() == TYPEPARAM && self.pos() < template_end {
                template.tparams.push(self.d.tparam());
            }
            while self.d.r.peek() == PARAM && self.pos() < template_end {
                template.params.push(self.d.param());
            }
            while !matches!(self.d.r.peek(), DEFDEF | SPLITCLAUSE | SELFDEF) && self.pos() < template_end {
                template.parents.push(self.read_term());
            }
            if self.d.r.peek() == SPLITCLAUSE {
                self.d.r.byte();
            }
            if self.d.r.peek() == SELFDEF {
                self.d.r.byte();
                let self_name = self.d.r.nat();
                let ty = self.d.read_tpt();
                template.self_type = Some((self_name, ty));
            }
            if self.d.r.peek() == DEFDEF && self.pos() < template_end {
                let ctor_addr = self.pos() as Addr;
                template.ctor = Some(self.def_with_body(ctor_addr));
            }
            template.stats = self.stats(template_end);
            self.d.r.pos = template_end;
        } else {
            self.d.skip_tree();
        }
        let mods = self.d.modifiers(end);
        ClassDef { addr, name, mods, template }
    }
}

/// Calls `f` on every term of a statement, its definitions' bodies included, with the address
/// of the innermost enclosing method (`DEFDEF`) and whether that method is `inline`.
pub struct Walk<'f> {
    pub f: &'f mut dyn FnMut(&Term, &Scope),
}

#[derive(Clone, Copy, Default)]
pub struct Scope {
    /// The `DEFDEF` whose body this term is in, if any.
    pub method: Option<Addr>,
    pub in_inline: bool,
    pub in_quote: bool,
}

impl<'f> Walk<'f> {
    pub fn stat(&mut self, s: &Stat, scope: Scope) {
        match s {
            Stat::Val(_, Some(rhs)) => self.term(rhs, scope),
            Stat::Def(sig, Some(rhs)) => {
                let inner = Scope { method: Some(sig.addr), in_inline: scope.in_inline || sig.mods.flags.has(INLINE), ..scope };
                self.term(rhs, inner)
            }
            Stat::Class(c) => self.template(&c.template, scope),
            Stat::Expr(t) => self.term(t, scope),
            _ => {}
        }
    }

    pub fn template(&mut self, t: &Template, scope: Scope) {
        for p in &t.parents {
            self.term(p, scope);
        }
        if let Some((sig, rhs)) = &t.ctor {
            if let Some(rhs) = rhs {
                self.term(rhs, Scope { method: Some(sig.addr), ..scope });
            }
        }
        for s in &t.stats {
            self.stat(s, scope);
        }
    }

    pub fn case(&mut self, c: &Case, scope: Scope) {
        self.term(&c.pat, scope);
        if let Some(g) = &c.guard {
            self.term(g, scope);
        }
        self.term(&c.body, scope);
    }

    pub fn term(&mut self, t: &Term, scope: Scope) {
        (self.f)(t, &scope);
        match &t.kind {
            TermKind::Select(q, _) | TermKind::SelectIn(q, ..) | TermKind::Throw(q) | TermKind::NamedArg(_, q) | TermKind::Typed(q, ..) => {
                self.term(q, scope)
            }
            TermKind::Apply(f, args) | TermKind::ApplySigPoly(f, _, args) => {
                self.term(f, scope);
                args.iter().for_each(|a| self.term(a, scope));
            }
            TermKind::TypeApply(f, ..) | TermKind::Lambda(f, _) | TermKind::Super(f, _) => self.term(f, scope),
            TermKind::Assign(a, b) | TermKind::While(a, b) => {
                self.term(a, scope);
                self.term(b, scope);
            }
            TermKind::Block(stats, expr) => {
                stats.iter().for_each(|s| self.stat(s, scope));
                self.term(expr, scope);
            }
            TermKind::If { cond, then, els, .. } => {
                self.term(cond, scope);
                self.term(then, scope);
                self.term(els, scope);
            }
            TermKind::Match { selector, cases, .. } => {
                if let Some(s) = selector {
                    self.term(s, scope);
                }
                cases.iter().for_each(|c| self.case(c, scope));
            }
            TermKind::Try { body, cases, finalizer } => {
                self.term(body, scope);
                cases.iter().for_each(|c| self.case(c, scope));
                if let Some(f) = finalizer {
                    self.term(f, scope);
                }
            }
            TermKind::Return { expr, .. } => {
                if let Some(e) = expr {
                    self.term(e, scope);
                }
            }
            TermKind::Inlined { expansion, call, bindings, .. } => {
                bindings.iter().for_each(|s| self.stat(s, scope));
                if let Some(c) = call {
                    self.term(c, scope);
                }
                self.term(expansion, scope);
            }
            TermKind::Repeated(_, elems) | TermKind::Alternative(elems) => elems.iter().for_each(|e| self.term(e, scope)),
            TermKind::SelectOuter { qual, .. } => self.term(qual, scope),
            TermKind::Quote { body, .. } => self.term(body, Scope { in_quote: true, ..scope }),
            TermKind::Splice { expr, .. } => self.term(expr, Scope { in_quote: false, ..scope }),
            TermKind::QuotePattern { body, quotes, bindings, .. } => {
                bindings.iter().for_each(|s| self.stat(s, scope));
                self.term(quotes, scope);
                self.term(body, Scope { in_quote: true, ..scope });
            }
            TermKind::SplicePattern { pat, args, .. } => {
                self.term(pat, scope);
                args.iter().for_each(|a| self.term(a, scope));
            }
            TermKind::Bind { body, .. } => self.term(body, scope),
            TermKind::Unapply { fun, implicits, pats, .. } => {
                self.term(fun, scope);
                implicits.iter().for_each(|a| self.term(a, scope));
                pats.iter().for_each(|a| self.term(a, scope));
            }
            TermKind::Hole { args, .. } => args.iter().for_each(|a| self.term(a, scope)),
            TermKind::Path(_)
            | TermKind::Const(_)
            | TermKind::Ident(..)
            | TermKind::QualThis(_)
            | TermKind::New(_)
            | TermKind::Elided(_)
            | TermKind::Unknown(..)
            | TermKind::TooDeep(_)
            | TermKind::Cycle(_) => {}
        }
    }
}

/// The name of a term's tag as `TastyFormat` spells it, for statistics.
pub fn tag_name(t: &Term) -> &'static str {
    match &t.kind {
        TermKind::Path(TType::LocalTerm(..)) => "TERMREFsymbol",
        TermKind::Path(TType::TermRef(..)) => "TERMREF",
        TermKind::Path(TType::This(_)) => "THIS",
        TermKind::Path(TType::Package(_)) => "TERMREFpkg",
        TermKind::Path(_) => "type tree",
        TermKind::Const(_) => "constant",
        TermKind::Ident(..) => "IDENT",
        TermKind::Select(..) => "SELECT",
        TermKind::SelectIn(..) => "SELECTin",
        TermKind::QualThis(_) => "QUALTHIS",
        TermKind::New(_) => "NEW",
        TermKind::Throw(_) => "THROW",
        TermKind::NamedArg(..) => "NAMEDARG",
        TermKind::Apply(..) => "APPLY",
        TermKind::ApplySigPoly(..) => "APPLYsigpoly",
        TermKind::TypeApply(..) => "TYPEAPPLY",
        TermKind::Super(..) => "SUPER",
        TermKind::Typed(..) => "TYPED",
        TermKind::Assign(..) => "ASSIGN",
        TermKind::Block(..) => "BLOCK",
        TermKind::If { inline: true, .. } => "IF inline",
        TermKind::If { .. } => "IF",
        TermKind::Lambda(..) => "LAMBDA",
        TermKind::Match { kind: MatchKind::Plain, .. } => "MATCH",
        TermKind::Match { kind: MatchKind::Inline, .. } => "MATCH inline",
        TermKind::Match { kind: MatchKind::Implicit, .. } => "MATCH implicit",
        TermKind::Match { kind: MatchKind::Sub, .. } => "MATCH sub",
        TermKind::Try { .. } => "TRY",
        TermKind::Return { .. } => "RETURN",
        TermKind::While(..) => "WHILE",
        TermKind::Inlined { .. } => "INLINED",
        TermKind::Repeated(..) => "REPEATED",
        TermKind::SelectOuter { .. } => "SELECTouter",
        TermKind::Quote { .. } => "QUOTE",
        TermKind::Splice { .. } => "SPLICE",
        TermKind::QuotePattern { .. } => "QUOTEPATTERN",
        TermKind::SplicePattern { .. } => "SPLICEPATTERN",
        TermKind::Bind { .. } => "BIND",
        TermKind::Alternative(_) => "ALTERNATIVE",
        TermKind::Unapply { .. } => "UNAPPLY",
        TermKind::Elided(_) => "ELIDED",
        TermKind::Hole { .. } => "HOLE",
        TermKind::Unknown(..) => "unknown",
        TermKind::TooDeep(_) => "too deep",
        TermKind::Cycle(_) => "cycle",
    }
}

impl<'a, 'd> TermDecoder<'a, 'd> {
    /// The number of bytes the tree at `addr` takes.
    pub fn tree_len(&mut self, addr: Addr) -> usize {
        self.d.r.pos = addr as usize;
        self.d.skip_tree();
        self.pos() - addr as usize
    }
}

/// The types a term node carries itself (those of its children are theirs).
pub fn node_types<'t>(t: &'t Term, out: &mut Vec<&'t TType>) {
    match &t.kind {
        TermKind::Path(ty) | TermKind::Ident(_, ty) | TermKind::QualThis(ty) | TermKind::New(ty) | TermKind::Elided(ty) => out.push(ty),
        TermKind::Const(Const::Class(ty)) => out.push(ty),
        TermKind::SelectIn(_, _, ty, _) | TermKind::Typed(_, ty, ..) => out.push(ty),
        TermKind::Repeated(ty, _) => out.push(ty),
        TermKind::ApplySigPoly(_, ty, _) => out.push(ty),
        TermKind::TypeApply(_, targs, ..) => out.extend(targs.iter()),
        TermKind::Super(_, Some(ty)) | TermKind::Lambda(_, Some(ty)) => out.push(ty),
        TermKind::SelectOuter { ty, .. } | TermKind::Quote { ty, .. } | TermKind::Splice { ty, .. } => out.push(ty),
        TermKind::QuotePattern { ty, .. } | TermKind::Unapply { ty, .. } | TermKind::Hole { ty, .. } | TermKind::Bind { ty, .. } => out.push(ty),
        TermKind::SplicePattern { ty, targs, .. } => {
            out.push(ty);
            out.extend(targs.iter());
        }
        _ => {}
    }
}

pub fn type_has_unknown(t: &TType) -> bool {
    match t {
        TType::Unknown(_) => true,
        TType::Package(_) | TType::RecThis(_) | TType::ParamRef(..) | TType::Const(_) => false,
        TType::TypeRef(p, _) | TType::TermRef(p, _) | TType::This(p) | TType::Alias(p) | TType::ByName(p) | TType::Rec(_, p) | TType::Flexible(p) => {
            type_has_unknown(p)
        }
        TType::LocalType(_, p) | TType::LocalTerm(_, p) => p.as_deref().map_or(false, type_has_unknown),
        TType::Applied(f, args) => type_has_unknown(f) || args.iter().any(type_has_unknown),
        TType::Bounds(a, b) | TType::BoundedAlias(a, b) | TType::And(a, b) | TType::Or(a, b) | TType::Annotated(a, b) | TType::Super(a, b) => {
            type_has_unknown(a) || type_has_unknown(b)
        }
        TType::Refined(p, members) => type_has_unknown(p) || members.iter().any(|(_, i)| i.as_ref().map_or(false, type_has_unknown)),
        TType::Lambda { params, result, .. } => params.iter().any(|p| type_has_unknown(&p.info)) || type_has_unknown(result),
        TType::Match { scrutinee, bound, cases } => {
            type_has_unknown(scrutinee)
                || bound.as_deref().map_or(false, type_has_unknown)
                || cases.iter().any(|c| type_has_unknown(&c.pattern) || type_has_unknown(&c.body))
        }
        TType::MatchCase(p, b) => type_has_unknown(p) || type_has_unknown(b),
    }
}

/// The qualified name of a class or object type, for statistics: `scala.quoted.Quotes.reflectModule`.
pub fn qualified_name(file: &super::TastyFile, d: &Decoder, t: &TType) -> String {
    match t {
        TType::Package(p) => file.name(*p),
        TType::TypeRef(prefix, n) | TType::TermRef(prefix, n) => {
            let p = qualified_name(file, d, prefix);
            let n = file.name(file.source_name(*n));
            if p.is_empty() { n } else { format!("{}.{}", p, n) }
        }
        TType::LocalType(addr, prefix) | TType::LocalTerm(addr, prefix) => {
            let n = d.name_at(*addr).map_or_else(|| format!("<local@{}>", addr), |n| file.name(file.source_name(n)));
            match prefix {
                Some(p) => {
                    let p = qualified_name(file, d, p);
                    if p.is_empty() { n } else { format!("{}.{}", p, n) }
                }
                None => n,
            }
        }
        TType::This(inner) | TType::Annotated(inner, _) | TType::Rec(_, inner) | TType::Flexible(inner) => qualified_name(file, d, inner),
        TType::Applied(f, _) => qualified_name(file, d, f),
        _ => String::new(),
    }
}

/// The bodies a statement holds, with the definition each belongs to: the right-hand sides of
/// vals and defs, and for a class its parent constructor calls, its constructor's body and the
/// statements of its template. Local definitions inside those bodies are theirs.
pub fn bodies<'s>(file: &super::TastyFile, s: &'s Stat, owner: &str, out: &mut Vec<(String, Option<&'s DefSig>, &'s Term)>) {
    match s {
        Stat::Val(sig, Some(rhs)) | Stat::Def(sig, Some(rhs)) => {
            out.push((format!("{}.{}", owner, file.name(sig.name)), Some(sig), rhs));
        }
        Stat::Class(c) => {
            let t = &c.template;
            let name = format!("{}.{}", owner, file.name(file.source_name(c.name)));
            for p in &t.parents {
                if !matches!(&p.kind, TermKind::Path(_)) {
                    out.push((name.clone(), None, p));
                }
            }
            if let Some((sig, Some(rhs))) = &t.ctor {
                out.push((name.clone(), Some(sig), rhs));
            }
            for s in &t.stats {
                match s {
                    Stat::Expr(e) => out.push((name.clone(), None, e)),
                    other => bodies(file, other, &name, out),
                }
            }
        }
        Stat::Expr(e) => out.push((owner.to_string(), None, e)),
        _ => {}
    }
}

impl TType {
    /// Whether the type names the local type at `addr` (a pattern's type variable).
    pub fn mentions_local(&self, addr: Addr) -> bool {
        match self {
            TType::LocalType(a, prefix) => *a == addr || prefix.as_deref().map_or(false, |p| p.mentions_local(addr)),
            TType::LocalTerm(_, prefix) => prefix.as_deref().map_or(false, |p| p.mentions_local(addr)),
            TType::TypeRef(p, _) | TType::TermRef(p, _) | TType::This(p) | TType::Alias(p) | TType::ByName(p) | TType::Flexible(p) => p.mentions_local(addr),
            TType::Rec(_, p) => p.mentions_local(addr),
            TType::Applied(f, args) => f.mentions_local(addr) || args.iter().any(|a| a.mentions_local(addr)),
            TType::Bounds(a, b) | TType::BoundedAlias(a, b) | TType::And(a, b) | TType::Or(a, b) | TType::Annotated(a, b) | TType::Super(a, b) | TType::MatchCase(a, b) => {
                a.mentions_local(addr) || b.mentions_local(addr)
            }
            TType::Refined(p, members) => p.mentions_local(addr) || members.iter().any(|(_, t)| t.as_ref().map_or(false, |t| t.mentions_local(addr))),
            TType::Lambda { params, result, .. } => params.iter().any(|p| p.info.mentions_local(addr)) || result.mentions_local(addr),
            TType::Match { scrutinee, bound, cases } => {
                scrutinee.mentions_local(addr)
                    || bound.as_deref().map_or(false, |b| b.mentions_local(addr))
                    || cases.iter().any(|c| c.pattern.mentions_local(addr) || c.body.mentions_local(addr))
            }
            TType::Package(_) | TType::RecThis(_) | TType::ParamRef(..) | TType::Const(_) | TType::Unknown(_) => false,
        }
    }
}
