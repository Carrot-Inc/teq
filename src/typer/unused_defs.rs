//! The unused definitions, scalac 3.8.4's `CheckUnused` for the kinds of `--wunused` beside
//! `imports`: privates, locals, explicit and implicit parameters, pattern variables, and the
//! variables never assigned or never read (E198, `unused private member`, `unused local
//! definition`, `unused explicit parameter`, ...).
//!
//! **Marks** (`refUsage`). A reference the typer commits to a definition of the program marks
//! it read, or assigned where it is an assignment's target (`isAssignment`): an expression's head
//! (`transformIdent`, `transformSelect`, `transformNew`: `Worker::head_reference`), a type the
//! source writes (`transformTypeTree`), a given a search took (an implicit argument's tree), an
//! inline method called (`transformInlined`'s call), a conversion applied, an extractor or a
//! constructor a pattern names. A reference inside the definition it names does not count
//! (`refUsage`'s enclosing owners): a method that calls only itself is unused. The marks are the
//! import check's (`unused.rs`), keyed by the definition's file and the start of its name, with
//! their lifetimes: an attempt abandoned takes its marks back, a retype redoes those of the
//! bodies and moves those of the signatures with the names.
//!
//! **The report** (`CheckUnused.warnings`) reads the definitions off each program file's tree
//! once the typing is over (`register`'s): the members of its classes and objects and its
//! top-level definitions, the definitions of its blocks, the parameters of its methods,
//! constructors and function literals, the variables its patterns bind; and reports each that
//! no mark reads by its kind, with dotty's exemptions: an override, a deprecated or `@unused`
//! owner, a method whose body consumes nothing (`isUnconsuming`: `???`, a literal, `{}`, `this`,
//! a value of type `Nothing`, an object), a `main`, a singleton-typed parameter, an implicit
//! parameter of a marker trait, `DummyImplicit`, `<:<` or `=:=`, a case class's field, a pattern
//! variable named as its field (`allowVariableBindings`), one a member binds. A read variable
//! never assigned is `unset`, an assigned one never read `mutated but not read`.

use super::referents::Referent;
use super::unused::Key;
use super::Worker;
use crate::ast::{self, mods, DefId, DefKind, Expr, ExprId, Pat, PatId, Stmt};
use crate::intern::{Interner, Name};
use crate::source::{FileId, Span};
use crate::symbols::{ClassKind, Owner, SymKind};
use crate::types::{AliasId, ClassId, SymId, TypeId};

/// What a parameter belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ParamOwner {
    /// A method, by its definition, and its symbol where it has one.
    Method(DefId, Option<SymId>),
    /// The primary constructor of a class.
    Constructor(DefId),
    /// A function literal.
    Lambda,
}

/// A definition the report reads off a file's tree.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// A member of a class or an object, or a top-level definition: effectively private where
    /// it is private or a member of an anonymous class that overrides nothing.
    Member { private: bool, anonymous: bool },
    /// A definition of a block.
    Local,
    Param { owner: ParamOwner, implicit: bool, index: usize },
    /// A variable a pattern binds, a member's (a pattern definition of a class body, never
    /// reported) or not, and allowed where it is named as the field it matches.
    PatVar { member: bool, allowed: bool },
}

#[derive(Clone, Copy, Debug)]
struct Registered {
    name: Name,
    /// The name's place, where the warning stands.
    at: Span,
    /// Where the definition's symbol stands, which its marks are keyed by: the name's start but
    /// for a context bound's evidence, whose symbol stands at its type parameter.
    key: u32,
    kind: Kind,
    def: Option<DefId>,
    mods: crate::ast::Mods,
    /// A `var`.
    mutable: bool,
    /// Annotated `@unused`.
    unused_annotated: bool,
    /// A case class's field, which its synthetic members read.
    case_accessor: bool,
    /// A parameter's default getter, which a call that leaves the argument out uses.
    default_getter: bool,
}

impl<'a> Worker<'a> {
    /// The reference the expression `e`, typed as `te`, makes: its referents marked.
    pub(super) fn use_expr(&mut self, e: ExprId, te: crate::tir::TExprId) {
        // The target of `x += e`, marked by the operation once it is typed: written where it is
        // an assignment, read where it is a method's receiver.
        if self.assign_op_target == Some(e) {
            return;
        }
        let Some(r) = self.head_reference(e, te) else { return };
        let assigned = r.assigned || self.unused.assign_target == Some(e);
        for referent in r.referents.into_iter().flatten() {
            self.use_referent(referent, r.span, assigned);
        }
    }

    /// `new C(..)` typed since the journal's `mark`, inside the class `C`: an argument that is
    /// a parameter of `C` passed to the constructor's parameter of its name is no use of it
    /// (CheckUnused's `ignoreArgsOfSelfConstruction`), its marks taken back where the
    /// construction reads it nowhere else.
    pub(super) fn ignore_self_construction_args(&mut self, te: crate::tir::TExprId, mark: usize) {
        use crate::tir::TExpr;
        let mut node = te;
        while let TExpr::Block(_, res) = self.prog.expr(node) {
            node = res;
        }
        let TExpr::New(c, args) = self.prog.expr(node) else { return };
        if !self.env.frames.iter().any(|f| matches!(f, super::Frame::Class(k) if *k == c)) {
            return;
        }
        // The constructor's parameters, a context bound's evidence among them.
        let params: Vec<SymId> = self.syms.class(c).ctor.iter().flat_map(|clause| clause.params.iter().map(|p| p.sym)).collect();
        let args = self.prog.expr_list(args).to_vec();
        // The captures and the outer instance a nested class takes come first.
        let Some(args) = args.len().checked_sub(params.len()).map(|skip| &args[skip..]) else { return };
        let mut ignored: Vec<(SymId, usize)> = Vec::new();
        for (&p, &a) in params.iter().zip(args) {
            let (TExpr::Field(_, s) | TExpr::Local(s)) = self.prog.expr(a) else { continue };
            if self.syms.sym(s).owner == Owner::Class(c) && (s == p || self.syms.sym(s).span == self.syms.sym(p).span) && self.syms.sym(s).name == self.syms.sym(p).name {
                match ignored.iter_mut().find(|(x, _)| *x == s) {
                    Some((_, n)) => *n += 1,
                    None => ignored.push((s, 1)),
                }
            }
        }
        for (s, n) in ignored {
            let reads = self.prog.descendants(node).filter(|&x| matches!(self.prog.expr(x), TExpr::Field(_, v) | TExpr::Local(v) if v == s)).count();
            if reads != n {
                continue;
            }
            let info = self.syms.sym(s);
            let key = Key::Read(info.file, self.unused.def_at.get(&s).copied().unwrap_or(info.span.start));
            self.unused.forget_since(mark, key);
        }
    }

    /// The symbol `sym` stands for the definition whose name is at `start` of its file.
    #[inline(always)]
    pub(super) fn note_def_at(&mut self, sym: SymId, start: u32) {
        if self.unused.defs_on() {
            self.unused.def_at.insert(sym, start);
        }
    }

    /// The symbol `sym` stands for `original` (a comprehension's variable restored from a
    /// pack).
    #[inline(always)]
    pub(super) fn note_stand_in(&mut self, sym: SymId, original: SymId) {
        if self.unused.defs_on() {
            let start = self.unused.def_at.get(&original).copied().unwrap_or(self.syms.sym(original).span.start);
            self.unused.def_at.insert(sym, start);
        }
    }

    /// `r` is referred to at `at` of the current file: read, or with `assigned` assigned.
    pub(super) fn use_referent(&mut self, r: Referent, at: Span, assigned: bool) {
        let (file, name_at, def, class_kind) = match r {
            Referent::Sym(s) => {
                let info = self.syms.sym(s);
                // An object's value stands for its class, whose name the report reads.
                if let SymKind::Object(c) = info.kind {
                    return self.use_referent(Referent::Class(c), at, assigned);
                }
                let start = self.unused.def_at.get(&s).copied().unwrap_or(info.span.start);
                (info.file, start, info.def, None)
            }
            Referent::Class(c) => {
                let info = self.syms.class(c);
                (info.file, info.span.start, info.def, Some(info.kind))
            }
            Referent::Alias(a) => return self.use_alias(a, at),
        };
        if (file.0 as usize) >= self.files.len() || self.files[file.0 as usize].is_std {
            return;
        }
        // A reference inside the definition it names: its own recursion, a class's mention of
        // itself.
        if let Some(d) = def.filter(|_| file == self.env.file && at != Span::default()) {
            let range = self.ast(file).def_range(d);
            if range.start <= at.start && at.end <= range.end && class_kind != Some(ClassKind::Object) {
                return;
            }
        }
        self.mark_key(if assigned { Key::Assign(file, name_at) } else { Key::Read(file, name_at) });
    }

    /// The symbol `s` is referred to at `at` of the current file, without an expression of its
    /// own: a given a search took, an inline method expanded, a conversion applied, an
    /// extractor a pattern calls.
    #[inline(always)]
    pub(super) fn use_sym(&mut self, s: SymId, at: Span) {
        if self.unused.defs_on() {
            self.use_referent(Referent::Sym(s), at, false);
        }
    }

    /// A call leaves out the argument of the parameter `param`: its default (getter) is used.
    #[inline(always)]
    pub(super) fn use_default(&mut self, param: SymId) {
        if self.unused.defs_on() {
            self.use_default_now(param);
        }
    }

    fn use_default_now(&mut self, param: SymId) {
        let info = self.syms.sym(param);
        let (file, start) = (info.file, info.span.start);
        if (file.0 as usize) < self.files.len() && !self.files[file.0 as usize].is_std {
            self.mark_key(Key::Default(file, start));
        }
    }

    /// The class `c` is named at `at` of the current file: in a type, a pattern.
    #[inline(always)]
    pub(super) fn use_class(&mut self, c: ClassId, at: Span) {
        if self.unused.defs_on() {
            self.use_referent(Referent::Class(c), at, false);
        }
    }

    /// The type alias `a` is named at `at`.
    #[inline(always)]
    pub(super) fn use_alias(&mut self, a: AliasId, at: Span) {
        if self.unused.defs_on() {
            self.use_alias_now(a, at);
        }
    }

    fn use_alias_now(&mut self, a: AliasId, at: Span) {
        let info = self.syms.alias(a);
        let (file, def) = (info.file, info.def);
        let Some(d) = def else { return };
        if (file.0 as usize) >= self.files.len() || self.files[file.0 as usize].is_std {
            return;
        }
        let ast = self.ast(file);
        let range = ast.def_range(d);
        if file == self.env.file && range.start <= at.start && at.end <= range.end {
            return;
        }
        let name_at = ast.def(d).span.start;
        self.mark_key(Key::Read(file, name_at));
    }

    /// The unused definitions of the program file `file` and its units `units` (`report_unused`),
    /// each with its message, in place order (`CheckUnused.warnings`).
    pub(super) fn unused_definitions(&mut self, units: &[FileId]) -> Vec<(FileId, Span, String)> {
        let kinds = self.unused.defs;
        let mut out: Vec<(FileId, Span, String)> = Vec::new();
        for &u in units {
            let mut defs: Vec<Registered> = Vec::new();
            register_file(self.ast(u), self.interner, &self.files[u.0 as usize].text, &mut defs);
            // The variables of one place: a pattern a comprehension repeats (CheckUnused's
            // `byPos`).
            defs.sort_by_key(|d| (d.at.start, d.at.end, d.key));
            defs.dedup_by_key(|d| (d.at.start, d.at.end, d.key));
            for d in defs {
                if d.unused_annotated {
                    continue;
                }
                let read = d.case_accessor || if d.default_getter { self.unused.has_default(u, d.key) } else { self.unused.has(u, d.key, false) };
                let assigned = self.unused.has(u, d.key, true);
                let msg = match d.kind {
                    Kind::PatVar { member, allowed } => {
                        if !kinds.patvars {
                            None
                        } else if !read {
                            (!member && !allowed).then_some("unused pattern variable")
                        } else if d.mutable && !assigned && !member {
                            Some("unset local variable, consider using an immutable val instead")
                        } else {
                            None
                        }
                    }
                    _ if read => self.check_unassigned(u, &d, assigned, kinds),
                    Kind::Member { private: true, anonymous } => {
                        // A member of an anonymous class is effectively private where it overrides
                        // nothing (`isEffectivelyPrivate`).
                        if anonymous && d.def.and_then(|id| self.def_syms.get(u.0 as usize, &id).copied()).is_some_and(|s| self.overrides_inherited(s)) {
                            None
                        } else {
                            self.check_private(u, &d, assigned, kinds)
                        }
                    }
                    Kind::Member { private: false, .. } => None,
                    Kind::Param { owner, implicit, index } => self.check_param(u, &d, owner, implicit, index, kinds),
                    Kind::Local => (kinds.locals && !is_synthetic_name(self.name_ref(d.name))).then(|| if d.mutable && assigned { "local variable was mutated but not read" } else { "unused local definition" }),
                };
                if let Some(m) = msg {
                    let mut text = m.to_string();
                    // `paramAddendum`: the extension method a parameter of an extension belongs to.
                    if let (Kind::Param { owner: ParamOwner::Method(md, _), .. }, true) = (d.kind, m.starts_with("unused")) {
                        let def = self.ast(u).def(md);
                        if matches!(&def.kind, DefKind::Fun(f) if f.is_extension) {
                            text.push_str(&format!(" in extension method {}", self.name_ref(def.name)));
                        }
                    }
                    out.push((u, d.at, text));
                }
            }
        }
        out.sort_by_key(|&(f, s, _)| (f.0, s.start));
        out
    }

    /// `checkUnassigned`: a read `var` that nothing assigns.
    fn check_unassigned(&self, _u: FileId, d: &Registered, assigned: bool, kinds: super::unused::DefKinds) -> Option<&'static str> {
        if !d.mutable || assigned {
            return None;
        }
        match d.kind {
            Kind::Local if kinds.locals => Some("unset local variable, consider using an immutable val instead"),
            Kind::Member { private: true, .. } if kinds.privates => Some("unset private variable, consider using an immutable val instead"),
            _ => None,
        }
    }

    /// `checkPrivate`.
    fn check_private(&mut self, u: FileId, d: &Registered, assigned: bool, kinds: super::unused::DefKinds) -> Option<&'static str> {
        if !kinds.privates {
            return None;
        }
        let name = self.name_ref(d.name);
        if is_synthetic_name(name) || (matches!(name, "readResolve" | "readObject" | "readObjectNoData" | "writeObject" | "writeReplace") && self.serializable_member(u, d)) {
            return None;
        }
        Some(if d.mutable && assigned { "private variable was mutated but not read" } else { "unused private member" })
    }

    /// Whether the member `d` belongs to a class that derives from `java.io.Serializable`.
    fn serializable_member(&mut self, u: FileId, d: &Registered) -> bool {
        let Some(sym) = d.def.and_then(|id| self.def_syms.get(u.0 as usize, &id).copied()) else { return false };
        let Owner::Class(c) = self.syms.sym(sym).owner else { return false };
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        bases.iter().any(|&b| self.interner.get(self.syms.class(b).name) == "Serializable")
    }

    /// `checkParam` and `checkImplicit`.
    fn check_param(&mut self, u: FileId, d: &Registered, owner: ParamOwner, implicit: bool, index: usize, kinds: super::unused::DefKinds) -> Option<&'static str> {
        if (implicit && !kinds.implicits) || (!implicit && !kinds.explicits && !matches!(owner, ParamOwner::Constructor(_))) {
            return None;
        }
        // A name the compiler made (`x$1` of `using Int`) is an explicit parameter's exemption
        // (`DerivedName`), not an implicit one's.
        if is_synthetic_name(self.name_ref(d.name)) && !implicit {
            return None;
        }
        let ast = self.ast(u);
        match owner {
            ParamOwner::Lambda => {
                // An implicit or contextual parameter of a function literal: its method is
                // synthetic (`allowed`).
                if implicit || !kinds.explicits {
                    return None;
                }
                Some("unused explicit parameter")
            }
            ParamOwner::Constructor(cd) => {
                let def = ast.def(cd);
                // A case class's fields are its accessors (`CaseAccessor`), a `val` one a member.
                if def.mods & mods::CASE != 0 && index_in_first_clause(def, index) {
                    return None;
                }
                if d.mods & (mods::FIELD) != 0 && d.mods & mods::PRIVATE == 0 {
                    return None;
                }
                if self.annotation_class_def(u, cd) || def.mods & mods::DEPRECATED != 0 {
                    return None;
                }
                if implicit {
                    if let Some(ty) = self.constructor_param_type(u, cd, index, d.key) {
                        if self.type_exempts(ty, true) {
                            return None;
                        }
                    }
                    return kinds.implicits.then_some("unused implicit parameter");
                }
                if d.mods & mods::FIELD != 0 {
                    // `private val x` of the constructor: a private member.
                    return kinds.privates.then_some("unused private member");
                }
                kinds.explicits.then_some("unused explicit parameter")
            }
            ParamOwner::Method(md, sym) => {
                let sym = sym.or_else(|| self.def_syms.get(u.0 as usize, &md).copied());
                let def = ast.def(md);
                if def.mods & (mods::DEPRECATED | mods::OVERRIDE) != 0 || def.annots.iter().any(|a| a.name == crate::names::UNUSED) {
                    return None;
                }
                let DefKind::Fun(f) = &def.kind else { return None };
                // A method whose body consumes nothing, an abstract one, a default getter.
                match f.body {
                    None => return None,
                    Some(b) if self.unconsuming(u, b, sym) => return None,
                    _ => {}
                }
                if let Some(s) = sym {
                    if self.overrides_inherited(s) {
                        return None;
                    }
                    if !implicit && self.is_java_main(s) && self.syms.sym(s).name == crate::names::MAIN {
                        return None;
                    }
                    if self.param_exempt(s, index, implicit, d.key) {
                        return None;
                    }
                }
                // The receiver of an extension method.
                if !implicit && f.is_extension && index == 0 && f.ext_clauses > 0 {
                    return None;
                }
                Some(if implicit { "unused implicit parameter" } else { "unused explicit parameter" })
            }
        }
    }

    /// The type of the parameter `index` of the primary constructor of the class `cd`, or with
    /// `usize::MAX` of the one whose symbol stands at `key` (a context bound's evidence).
    fn constructor_param_type(&mut self, u: FileId, cd: DefId, index: usize, key: u32) -> Option<TypeId> {
        let &c = self.def_classes.get(u.0 as usize, &cd)?;
        self.complete_class(c);
        let info = self.syms.class(c);
        let params: Vec<(TypeId, SymId)> = info.ctor.iter().flat_map(|cl| cl.params.iter().map(|p| (p.ty, p.sym))).collect();
        match index {
            usize::MAX => params.into_iter().find(|&(_, s)| self.syms.sym(s).span.start == key).map(|(t, _)| t),
            _ => params.get(index).map(|&(t, _)| t),
        }
    }

    /// Whether the definition `cd` of `u` is a class that derives from `Annotation`, whose
    /// constructor's parameters are its arguments.
    fn annotation_class_def(&mut self, u: FileId, cd: DefId) -> bool {
        let Some(&c) = self.def_classes.get(u.0 as usize, &cd) else { return false };
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        bases.iter().any(|&b| self.interner.get(self.syms.class(b).name) == "Annotation")
    }

    /// Whether the method `m` overrides or implements an inherited member
    /// (`isEffectivelyOverride`).
    fn overrides_inherited(&mut self, m: SymId) -> bool {
        if self.syms.sym(m).mods & mods::OVERRIDE != 0 {
            return true;
        }
        let Owner::Class(c) = self.syms.sym(m).owner else { return false };
        let name = self.syms.sym(m).name;
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).filter(|&b| b != c).collect();
        bases.into_iter().any(|b| {
            self.complete_class(b);
            self.syms.class(b).members.get(&name).is_some_and(|&s| self.syms.sym(s).mods & mods::PRIVATE == 0)
        })
    }

    /// The parameter `index` of the method `m`, exempt by its type: a singleton, and for an
    /// implicit one `DummyImplicit`, `<:<`, `=:=`, a marker trait (no member of its own but
    /// `Any`'s and `Object`'s), a refinement (`checkImplicit`'s `allowed`).
    fn param_exempt(&mut self, m: SymId, index: usize, implicit: bool, key: u32) -> bool {
        let sig = self.sig_arc(m);
        let found = match index {
            usize::MAX => sig.clauses.iter().flat_map(|c| c.params.iter()).find(|p| self.syms.sym(p.sym).span.start == key).map(|p| p.ty),
            _ => sig.clauses.iter().flat_map(|c| c.params.iter()).nth(index).map(|p| p.ty),
        };
        let Some(ty) = found else { return false };
        self.type_exempts(ty, implicit)
    }

    /// `param_exempt` of a parameter of type `ty`.
    fn type_exempts(&mut self, ty: TypeId, implicit: bool) -> bool {
        let ty = self.dealias(ty);
        // `Any`, every member of which is `Any`'s, is a marker (`isMarkerTrait`).
        if implicit && ty == crate::types::ANY {
            return true;
        }
        if matches!(self.types.get(ty), crate::types::Type::Term(_) | crate::types::Type::This(_)) {
            return true;
        }
        if !implicit {
            return false;
        }
        match self.types.get(ty) {
            crate::types::Type::Refined(..) => true,
            // An abstract type is a marker where its upper bound is.
            crate::types::Type::Member(..) | crate::types::Type::AppMember(..) | crate::types::Type::Decl(_) => {
                let (_, hi) = self.member_bounds(ty);
                let hi = self.dealias(hi);
                match self.types.get(hi) {
                    crate::types::Type::Class(c, _) => self.marker_trait(c),
                    _ => hi == crate::types::ANY,
                }
            }
            crate::types::Type::Class(c, _) => {
                let name = self.interner.get(self.syms.class(c).name);
                matches!(name, "DummyImplicit" | "<:<" | "=:=") || self.marker_trait(c)
            }
            _ => false,
        }
    }

    /// Whether the class `c` has no term member but those of `Any` and `Object`
    /// (`isMarkerTrait`).
    fn marker_trait(&mut self, c: ClassId) -> bool {
        // A primitive's members and a string's are the typer's own, which their tables do not
        // list.
        let b = &self.b;
        if [b.int, b.long, b.double, b.float, b.short, b.byte, b.char, b.boolean, b.unit, b.string].contains(&c) {
            return false;
        }
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        for b in bases {
            let name = self.interner.get(self.syms.class(b).name);
            if matches!(name, "Any" | "Object" | "AnyRef" | "Matchable") {
                continue;
            }
            if !self.syms.class(b).members.is_empty() {
                return false;
            }
        }
        true
    }

    /// `isUnconsuming`: the body `b` of the method `m` uses none of its parameters by its very
    /// form: `???`, a literal or a constant, `{}`, `this`, a value of type `Nothing` (a throw, a
    /// call of a method of that result), an object, a parameter accessor of the class, an
    /// ascription of one; read off the body as typed where it is, as written otherwise.
    fn unconsuming(&mut self, u: FileId, b: ExprId, m: Option<SymId>) -> bool {
        let ast = self.ast(u);
        match ast.expr(b) {
            Expr::IntLit(_) | Expr::LongLit(_) | Expr::DoubleLit(_) | Expr::DecimalLit(_) | Expr::FloatLit(_) | Expr::BoolLit(_) | Expr::CharLit(_) | Expr::StringLit(_) | Expr::UnitLit | Expr::NullLit => return true,
            Expr::This | Expr::Throw(_) => return true,
            Expr::Typed(x, _) | Expr::Parens(x) => return self.unconsuming(u, x, None) || m.is_some_and(|m| self.unconsuming_typed(m)),
            Expr::Block(stmts) if ast.stmt_list(stmts).is_empty() => return true,
            Expr::Ident(n) | Expr::Select(_, n) if self.interner.get(n) == "???" => return true,
            _ => {}
        }
        m.is_some_and(|m| self.unconsuming_typed(m))
    }

    /// `isUnconsuming` of the typed body of the method `m`.
    fn unconsuming_typed(&mut self, m: SymId) -> bool {
        let Some(&f) = self.fun_of_sym.get(&m) else { return false };
        let (Some(body), body_unconsuming) = ({ let fun = self.prog.funs.get(f.0); (fun.body, fun.body_unconsuming) }) else { return false };
        use crate::tir::TExpr;
        // The typed right-hand side's type, a constant type (`rhs.tpe match case ConstantType`):
        // its own, before its adaptation to the declared result (`one(2)` of `def one(i: Int):
        // 1`, whatever `one` does).
        // A body of type `Nothing` (`rhs.tpe =:= NothingType`: a throw, `compiletime.error`).
        if body_unconsuming {
            return true;
        }
        // teq's typer gives a literal, a constant member and an operation over constants the
        // widened type where scalac's gives them a `ConstantType`: that type is read off the
        // typed result of the body's blocks, whatever their statements do (`typed_constant`),
        // an ascription widening it (`{ println(); 1 }` exempt, `{ println(); (1: Int) }` not).
        // An `if` on a pure constant is its branch by then (`FirstTransform.transformIf`).
        let mut res = body;
        loop {
            match self.prog.expr(res) {
                TExpr::Block(_, r) => res = r,
                TExpr::If(c, t, e) => match self.folded_condition(c) {
                    Some(true) => res = t,
                    Some(false) => match e {
                        Some(e) => res = e,
                        None => break,
                    },
                    None => break,
                },
                _ => break,
            }
        }
        if self.typed_constant(res).is_some() && !self.widened_constant(res) {
            return true;
        }
        let mut body = body;
        while let TExpr::Block(stmts, r) = self.prog.expr(body) {
            if !stmts.is_empty() {
                break;
            }
            body = r;
        }
        match self.prog.expr(body) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null => true,
            TExpr::This | TExpr::Module(_) | TExpr::Throw(..) | TExpr::ClassOf(_) => true,
            TExpr::Block(stmts, res) => stmts.is_empty() && matches!(self.prog.expr(res), TExpr::Unit),
            TExpr::Static(s) => matches!(self.syms.sym(s).kind, SymKind::Object(_) | SymKind::EnumValue(_)),
            // An object of the class, read through its accessor.
            TExpr::Field(_, s) | TExpr::CallMethod(_, s, _) if self.names_object(s) => true,
            // A parameter accessor of the class (`Ident(_) => rhs.symbol.is(ParamAccessor)`).
            TExpr::Field(r, s) => matches!(self.prog.expr(r), TExpr::This) && self.is_param_accessor(s),
            _ => false,
        }
    }

    /// The constant type scalac's typer gives the typed tree `e` where teq's type of it is the
    /// widened one: a literal's and a constant member's, as the typer folded them; the literal
    /// type a call or a member is declared with (`constant_by_type`); `ConstFold`'s over operands
    /// that have one (`answer + 27` of `def answer: 42`, whatever `answer` does), as its
    /// `ConstantTree` reads each operand's type. Never what evaluating the tree would give.
    fn typed_constant(&mut self, e: crate::tir::TExprId) -> Option<crate::types::LitVal> {
        use crate::tir::TExpr;
        match self.prog.expr(e) {
            TExpr::Prim(op, a, b) => {
                let (x, y) = (self.typed_constant(a)?, self.typed_constant(b)?);
                super::inline::fold_prim(op, x, y)
            }
            _ => self.fold_constant_as_typed(e).or_else(|| self.constant_by_type(e, None)),
        }
    }

    /// Whether the member `s` is an object, or the value an object nested in a class is read
    /// through.
    fn names_object(&mut self, s: SymId) -> bool {
        if matches!(self.syms.sym(s).kind, SymKind::Object(_)) {
            return true;
        }
        let ret = self.sig_arc(s).ret;
        let ret = self.deref(ret);
        match self.types.get(ret) {
            crate::types::Type::Class(c, _) => self.syms.class(c).kind == ClassKind::Object || self.syms.class(c).inner_object == Some(s),
            _ => false,
        }
    }

    /// Whether `s` is a parameter of its class's constructor, which the class reads as a field.
    fn is_param_accessor(&self, s: SymId) -> bool {
        let Owner::Class(c) = self.syms.sym(s).owner else { return false };
        self.syms.class(c).ctor_syms.iter().flatten().any(|&p| p == s || self.syms.sym(p).span == self.syms.sym(s).span)
    }
}

/// Whether a name is one the compiler makes (`x$1`, an evidence's), which no warning names.
fn is_synthetic_name(name: &str) -> bool {
    name.contains('$')
}

/// Whether the `index`-th parameter of the class `def` stands in its first parameter list.
fn index_in_first_clause(def: &ast::Def, index: usize) -> bool {
    let DefKind::Class(cls) = &def.kind else { return false };
    cls.clauses.first().is_some_and(|c| index < c.params.len())
}

/// Every definition of the file's tree (`register`): its members, its blocks' definitions,
/// its parameters and its pattern variables.
fn register_file(ast: &ast::Ast, names: &Interner, text: &str, out: &mut Vec<Registered>) {
    let r = Registrar { ast, names, text };
    for &d in &ast.top_level {
        r.def(d, Context::TopLevel, out);
    }
}

#[derive(Clone, Copy)]
enum Context {
    /// A top-level definition, a member of its file's package object: a private one is visible
    /// in the whole package and never reported.
    TopLevel,
    Member { anonymous: bool },
    Local,
}

/// The walk of one file's tree.
struct Registrar<'t> {
    ast: &'t ast::Ast,
    names: &'t Interner,
    text: &'t str,
}

impl<'t> Registrar<'t> {
    /// The place of the name `name` written at `start`: after a backquote, the name's length long.
    fn name_at(&self, start: u32, name: Name) -> Span {
        let start = if self.text.as_bytes().get(start as usize) == Some(&b'`') { start + 1 } else { start };
        Span::new(start, start + self.names.get(name).chars().count() as u32)
    }

    /// A definition whose symbol stands at `key`, reported at `at`.
    #[allow(clippy::too_many_arguments)]
    fn reg(&self, out: &mut Vec<Registered>, name: Name, key: u32, at: Span, kind: Kind, def: Option<DefId>, m: crate::ast::Mods, unused_annotated: bool) {
        out.push(Registered { name, at, key, kind, def, mods: m, mutable: m & mods::MUTABLE != 0, unused_annotated, case_accessor: false, default_getter: false });
    }

    /// The parameters of the clauses `clauses` and the evidence of the context bounds of
    /// `tparams` of the method or class `d`.
    fn params(&self, d: DefId, clauses: &[ast::ParamClause], tparams: &[ast::TypeParam], ctor: Option<bool>, getters_private: bool, out: &mut Vec<Registered>) {
        let ast = self.ast;
        let mut index = 0;
        for (ci, c) in clauses.iter().enumerate() {
            for p in &c.params {
                let implicit = c.is_using || c.is_implicit;
                let annotated = ast.param_annots(p).iter().any(|a| a.name == crate::names::UNUSED);
                // An anonymous `using` parameter (`using Int`) stands at its type, one wide.
                let at = if self.names.get(p.name).contains('$') { Span::new(p.span.start, p.span.start + 1) } else { self.name_at(p.span.start, p.name) };
                let kind = match ctor {
                    // A `val` or `var` parameter is a field, a member of the class: a private one
                    // is checked as a private member (`PrivateParamAccessor`), a public one and a
                    // case class's field (`CaseAccessor`) never.
                    Some(_) if p.mods & mods::FIELD != 0 => {
                        let qualified = ast.access_scopes.iter().any(|&(at, _)| at == p.span.start);
                        Kind::Member { private: p.mods & mods::PRIVATE != 0 && !qualified, anonymous: false }
                    }
                    Some(_) => Kind::Param { owner: ParamOwner::Constructor(d), implicit, index },
                    None => Kind::Param { owner: ParamOwner::Method(d, None), implicit, index },
                };
                self.reg(out, p.name, p.span.start, at, kind, None, p.mods, annotated);
                // A case class's first parameter list are its fields (`CaseAccessor`), which the
                // synthetic members read.
                if ctor == Some(true) && ci == 0 {
                    if let Some(last) = out.last_mut() {
                        last.case_accessor = true;
                    }
                }
                index += 1;
            }
        }
        // A context bound's evidence, an implicit parameter at the end of the bound, whose
        // symbol stands at the type parameter (`resolve.rs`).
        for tp in tparams {
            for &b in &tp.context_bounds {
                let end = ast.ty_spans[b.idx()].end;
                let owner = if ctor.is_some() { ParamOwner::Constructor(d) } else { ParamOwner::Method(d, None) };
                out.push(Registered { name: tp.name, at: Span::new(end, end + 1), key: tp.span.start, kind: Kind::Param { owner, implicit: true, index: usize::MAX }, def: None, mods: 0, mutable: false, unused_annotated: false, case_accessor: false, default_getter: false });
            }
        }
        // A method's default getters, members as private as the method, at their defaults one
        // method's name long (`bippy$default$2` of `bippy`).
        let def = ast.def(d);
        let private = getters_private && ctor.is_none();
        for c in clauses {
            for p in &c.params {
                if let Some(e) = p.default {
                    if private {
                        let start = ast.expr_span(e).start;
                        let at = Span::new(start, start + self.names.get(def.name).chars().count() as u32);
                        out.push(Registered { name: def.name, at, key: p.span.start, kind: Kind::Member { private: true, anonymous: false }, def: None, mods: 0, mutable: false, unused_annotated: false, case_accessor: false, default_getter: true });
                    }
                    self.expr(e, out);
                }
            }
        }
    }

    fn def(&self, d: DefId, ctx: Context, out: &mut Vec<Registered>) {
        let ast = self.ast;
        let def = ast.def(d);
        let unused_annotated = def.annots.iter().any(|a| a.name == crate::names::UNUSED);
        // `private[C]` is no `Private` in scalac: only `private` and `private[this]` are.
        let qualified = ast.access_scopes.iter().any(|&(at, _)| at == def.span.start);
        let private = def.mods & mods::PRIVATE != 0 && def.mods & mods::QUALIFIED == 0 && !qualified;
        let kind = match ctx {
            Context::TopLevel => Kind::Member { private: false, anonymous: false },
            Context::Member { anonymous } => Kind::Member { private: private || (anonymous && def.mods & mods::OVERRIDE == 0), anonymous },
            Context::Local => Kind::Local,
        };
        // An anonymous given stands at its type, its name the one scalac makes of it.
        let at = match &def.kind {
            DefKind::Given(g) if def.mods & mods::ANONYMOUS != 0 => {
                let start = ast.ty_spans[g.ty.idx()].start;
                Span::new(start, start + self.names.get(def.name).chars().count() as u32)
            }
            // A secondary constructor stands at `this`.
            DefKind::Fun(_) if def.name == crate::names::INIT => Span::new(def.span.start, def.span.start + 4),
            _ => self.name_at(def.span.start, def.name),
        };
        let own = |out: &mut Vec<Registered>| self.reg(out, def.name, def.span.start, at, kind, Some(d), def.mods, unused_annotated);
        match &def.kind {
            DefKind::Val { pat: Some(p), rhs, .. } => {
                self.pat(*p, matches!(ctx, Context::Member { .. }), out);
                if let Some(r) = rhs {
                    self.expr(*r, out);
                }
            }
            DefKind::Val { rhs, ty, .. } => {
                if def.name != crate::names::WILDCARD {
                    own(out);
                }
                if let Some(r) = rhs {
                    let from = out.len();
                    self.expr(*r, out);
                    self.relax(*ty, *r, from, out);
                }
            }
            // An inline method is never registered, nor are its parameters (`inliners`).
            DefKind::Fun(f) => {
                if def.mods & mods::INLINE != 0 {
                    return;
                }
                own(out);
                self.params(d, &f.clauses, &f.tparams, None, matches!(kind, Kind::Member { private: true, .. }), out);
                if let Some(b) = f.body {
                    let from = out.len();
                    self.expr(b, out);
                    self.relax(f.ret, b, from, out);
                }
            }
            DefKind::Class(cls) => {
                own(out);
                self.params(d, &cls.clauses, &cls.tparams, Some(def.mods & mods::CASE != 0), false, out);
                for parent in &cls.parents {
                    for (args, _) in &parent.args {
                        for &a in ast.expr_list(*args) {
                            self.expr(a, out);
                        }
                    }
                }
                self.stmts(&cls.body, Context::Member { anonymous: false }, out);
            }
            DefKind::Given(g) => {
                own(out);
                self.params(d, &g.clauses, &g.tparams, None, false, out);
                if let Some(a) = g.alias {
                    let from = out.len();
                    self.expr(a, out);
                    self.relax(Some(g.ty), a, from, out);
                }
                self.stmts(&g.body, Context::Member { anonymous: false }, out);
            }
            DefKind::TypeAlias { .. } => own(out),
        }
    }

    /// `relax`: the members the type of a definition leaks, which are no warning's: those a
    /// written refinement names, every one of an anonymous class the definition's inferred type
    /// is (`new T { def f = 1 }` without a written type), among what its right-hand side
    /// registered from `from` on.
    fn relax(&self, ty: Option<ast::TyExprId>, rhs: ExprId, from: usize, out: &mut [Registered]) {
        let ast = self.ast;
        let names: Option<Vec<Name>> = match ty.map(|t| ast.ty(t)) {
            Some(ast::TyExpr::Refined(_, defs)) => Some(ast.def_list(defs).iter().map(|&d| ast.def(d).name).collect()),
            Some(_) => return,
            None => {
                let mut e = rhs;
                loop {
                    match ast.expr(e) {
                        Expr::Block(stmts) => match ast.stmt_list(stmts).last() {
                            Some(Stmt::Expr(last)) => e = *last,
                            _ => return,
                        },
                        Expr::Parens(x) => e = x,
                        Expr::NewAnon(_) => break,
                        _ => return,
                    }
                }
                None
            }
        };
        for r in &mut out[from..] {
            if matches!(r.kind, Kind::Member { anonymous: true, .. }) && names.as_ref().is_none_or(|n| n.contains(&r.name)) {
                r.unused_annotated = true;
            }
        }
    }

    fn stmts(&self, stmts: &[Stmt], ctx: Context, out: &mut Vec<Registered>) {
        for s in stmts {
            match *s {
                Stmt::Def(d) => self.def(d, ctx, out),
                Stmt::Expr(e) => self.expr(e, out),
                Stmt::Import(_) => {}
            }
        }
    }

    /// The definitions an expression holds: its blocks', its function literals' parameters, its
    /// patterns' variables, its anonymous classes' members.
    fn expr(&self, e: ExprId, out: &mut Vec<Registered>) {
        let ast = self.ast;
        let list = |l: ast::ListRef| ast.expr_list(l).to_vec();
        match ast.expr(e) {
            Expr::Block(stmts) => {
                let stmts = ast.stmt_list(stmts).to_vec();
                self.stmts(&stmts, Context::Local, out);
            }
            Expr::Lambda(params, body) => {
                for p in &ast.lambda_params[params.range()] {
                    let name = self.names.get(p.name);
                    // A placeholder's parameter (`_ + 1`) and `_` are the compiler's names.
                    if name.starts_with('_') || name.contains('$') {
                        continue;
                    }
                    let at = self.name_at(p.span.start, p.name);
                    self.reg(out, p.name, p.span.start, at, Kind::Param { owner: ParamOwner::Lambda, implicit: p.implicit || p.contextual, index: 0 }, None, 0, false);
                }
                self.expr(body, out);
            }
            Expr::PolyLambda(_, body) => self.expr(body, out),
            Expr::Match(s, cases) | Expr::InlineMatch(s, cases) => {
                self.expr(s, out);
                let from = out.len();
                self.cases(cases, out);
                // A case's binder named as the scrutinee it matches (`x match case x: 1`) is
                // allowed (`allowVariableBindings` of the selector's name).
                if let Expr::Ident(n) = ast.expr(s) {
                    let tops: Vec<u32> = ast.case_list(cases).iter().filter_map(|c| match ast.pat(c.pat) {
                        Pat::Bind(b, _) if b == n => Some(ast.pat_spans[c.pat.idx()].start),
                        Pat::Typed(p, _) => match ast.pat(p) {
                            Pat::Bind(b, _) if b == n => Some(ast.pat_spans[p.idx()].start),
                            _ => None,
                        },
                        _ => None,
                    }).collect();
                    for r in &mut out[from..] {
                        if matches!(r.kind, Kind::PatVar { .. }) && tops.contains(&r.key) {
                            r.kind = Kind::PatVar { member: false, allowed: true };
                        }
                    }
                }
            }
            Expr::Try(i) => {
                let t = ast.try_expr(i);
                self.expr(t.body, out);
                self.cases(t.cases, out);
                for x in [t.handler, t.finalizer].into_iter().flatten() {
                    self.expr(x, out);
                }
            }
            Expr::For(enums, body, _) => {
                let enums = &ast.enumerators[enums.range()];
                for (i, en) in enums.iter().enumerate() {
                    match *en {
                        // A generator's plain binder becomes a function literal's parameter, which
                        // the comprehension makes (`ForArtifact`), but where a value definition
                        // follows it: the desugaring tuples it into a case pattern's variable.
                        ast::Enumerator::Gen(p, x) | ast::Enumerator::CaseGen(p, x) | ast::Enumerator::Val(p, x) => {
                            let plain = matches!(ast.pat(p), Pat::Bind(_, None) | Pat::Wildcard);
                            let tupled = matches!(en, ast::Enumerator::Gen(..) | ast::Enumerator::CaseGen(..)) && enums[i + 1..].iter().take_while(|e| !matches!(e, ast::Enumerator::Gen(..) | ast::Enumerator::CaseGen(..))).any(|e| matches!(e, ast::Enumerator::Val(..)));
                            if !plain || tupled {
                                self.pat(p, false, out);
                            }
                            self.expr(x, out);
                        }
                        ast::Enumerator::Guard(x) => self.expr(x, out),
                    }
                }
                self.expr(body, out);
            }
            Expr::NewAnon(d) => {
                if let DefKind::Class(cls) = &ast.def(d).kind {
                    for parent in &cls.parents {
                        for (args, _) in &parent.args {
                            for &a in ast.expr_list(*args) {
                                self.expr(a, out);
                            }
                        }
                    }
                    self.stmts(&cls.body, Context::Member { anonymous: true }, out);
                }
            }
            Expr::Select(q, _) | Expr::TypeApply(q, _) | Expr::NamedArg(_, q) | Expr::Prefix(_, q) => self.expr(q, out),
            Expr::Apply(f, args) | Expr::UsingApply(f, args) => {
                self.expr(f, out);
                for a in list(args) {
                    self.expr(a, out);
                }
            }
            Expr::Infix(a, _, b) | Expr::While(a, b) | Expr::Assign(a, b) => {
                self.expr(a, out);
                self.expr(b, out);
            }
            Expr::If(c, t, e2) | Expr::InlineIf(c, t, e2) => {
                self.expr(c, out);
                self.expr(t, out);
                if let Some(e2) = e2 {
                    self.expr(e2, out);
                }
            }
            Expr::Tuple(items) | Expr::New(_, items) | Expr::Interp(_, _, items) | Expr::NamedTuple(_, items) => {
                for a in list(items) {
                    self.expr(a, out);
                }
            }
            Expr::Parens(x) | Expr::Typed(x, _) | Expr::Unchecked(x) | Expr::Throw(x) | Expr::Quote(x) | Expr::Splice(x) => self.expr(x, out),
            Expr::Return(Some(x)) => self.expr(x, out),
            _ => {}
        }
    }

    fn cases(&self, cases: ast::ListRef, out: &mut Vec<Registered>) {
        for c in self.ast.case_list(cases).to_vec() {
            self.pat(c.pat, false, out);
            if let Some(g) = c.guard {
                self.expr(g, out);
            }
            self.expr(c.body, out);
        }
    }

    /// The variables the pattern `p` binds, a member's where `member`.
    fn pat(&self, p: PatId, member: bool, out: &mut Vec<Registered>) {
        let ast = self.ast;
        match ast.pat(p) {
            Pat::Bind(n, inner) => {
                let name = self.names.get(n);
                if name != "_" && !name.contains('$') {
                    let start = ast.pat_spans[p.idx()].start;
                    let at = self.name_at(start, n);
                    self.reg(out, n, start, at, Kind::PatVar { member, allowed: false }, None, 0, false);
                }
                if let Some(i) = inner {
                    self.pat(i, member, out);
                }
            }
            Pat::Typed(i, _) | Pat::Rest(i) | Pat::NamedField(_, i) => self.pat(i, member, out),
            Pat::Ctor(_, args) | Pat::Tuple(args) | Pat::Alt(args) | Pat::Interp(_, _, args) => {
                for &a in ast.pat_list(args) {
                    self.pat(a, member, out);
                }
            }
            _ => {}
        }
    }
}
