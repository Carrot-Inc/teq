//! Givens the compiler builds at the call site whenever no ordinary given is in scope: the
//! evidence `<:<` / `=:=`, `NotGiven`, `ClassTag` and `ValueOf`.
//! Also the definitions around the site, which `Symbol.spliceOwner` of a macro shows.

use super::Worker;
use crate::ast::ListRef;
use crate::intern::Name;
use crate::names;
use crate::source::Span;
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;

/// A definition around a macro call, as the reflect API shows it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SiteOwner {
    /// A val, def or given, a member or a local one.
    Sym(SymId),
    /// A val whose symbol is made after its right-hand side: a binder of a pattern definition.
    Binder(Name, Span),
    /// The synthetic val that holds the tuple of a pattern definition with several binders.
    PatternTemp(Span),
    /// The `$anonfun` of a lambda.
    Lambda(Span),
    /// scalac's local dummy, the owner of the statements of a class body.
    Dummy(ClassId),
}

#[derive(Default, Clone)]
pub struct DefSites {
    /// What scalac's owner chain holds at this point of the class or member being typed,
    /// outermost first, for `Symbol.spliceOwner` of a macro expanded here.
    pub owners: Vec<SiteOwner>,
    /// `scala.util.NotGiven`.
    pub(super) not_given_class: Option<ClassId>,
    /// `scala.reflect.ClassTag`.
    pub class_tag: Option<ClassId>,
    /// `scala.reflect.TypeTest`, the std's or a jar's, once a search has met it.
    pub(super) type_test: Option<ClassId>,
}

/// A type's erasure as a `ClassTag` of the type needs it: a class or one of the two types at
/// the bottom, with the array dimensions around it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Erased {
    Nothing(u8),
    Null(u8),
    Of(ClassId, u8),
}

impl Erased {
    fn dims(self) -> u8 {
        match self {
            Erased::Nothing(d) | Erased::Null(d) | Erased::Of(_, d) => d,
        }
    }

    fn with_dims(self, dims: u8) -> Erased {
        match self {
            Erased::Nothing(_) => Erased::Nothing(dims),
            Erased::Null(_) => Erased::Null(dims),
            Erased::Of(c, _) => Erased::Of(c, dims),
        }
    }

    fn element(self) -> Erased {
        self.with_dims(self.dims() - 1)
    }
}

impl<'a> Worker<'a> {
    pub fn find_site_classes(&mut self) {
        self.sites.not_given_class = self.entered_class_at(&["scala", "util", "NotGiven"]);
        self.sites.class_tag = self.entered_class_at(&["scala", "reflect", "ClassTag"]);
    }

    /// The class or object at a path of packages, then of classes nested in one another,
    /// entering the std file that defines it when first asked.
    pub fn class_at(&mut self, path: &[&str]) -> Option<ClassId> {
        self.class_at_path(path, true)
    }

    /// `class_at` among what is entered.
    fn entered_class_at(&mut self, path: &[&str]) -> Option<ClassId> {
        self.class_at_path(path, false)
    }

    fn class_at_path(&mut self, path: &[&str], demand: bool) -> Option<ClassId> {
        let mut p = ROOT_PKG;
        let mut i = 0;
        while i + 1 < path.len() {
            let name = self.interner.intern(path[i]);
            let sub = if demand { self.demand_pkg(p, name) } else { self.syms.pkg(p).entries.get(&name).and_then(|e| e.pkg) };
            match sub {
                Some(sub) => {
                    p = sub;
                    i += 1;
                }
                None => break,
            }
        }
        let name = self.interner.intern(path[i]);
        if demand {
            self.demand_std(p, name, crate::stdindex::TYPE | crate::stdindex::TERM);
        }
        let entry = self.syms.pkg(p).entries.get(&name)?;
        let mut class = entry.class.or_else(|| match entry.term.map(|s| self.syms.sym(s).kind) {
            Some(SymKind::Object(c)) => Some(c),
            _ => None,
        })?;
        i += 1;
        while i < path.len() {
            let name = self.interner.intern(path[i]);
            self.complete_class(class);
            let nested = self.syms.class(class).nested.get(&name).copied();
            let in_companion = self.syms.class(class).companion.and_then(|k| self.syms.class(k).nested.get(&name).copied());
            class = nested.or(in_companion)?;
            i += 1;
        }
        Some(class)
    }

    /// Whether `c` is `scala.util.NotGiven`: the class found at the start where the std defines
    /// it, or a jar's, entered later than `find_site_classes` ran, recognised by its path once.
    pub fn is_not_given_class(&mut self, c: ClassId) -> bool {
        if let Some(n) = self.sites.not_given_class {
            return n == c;
        }
        let name = self.syms.class(c).name;
        if self.name_str(name) != "NotGiven" || self.class_path(c) != "scala.util.NotGiven" {
            return false;
        }
        self.sites.not_given_class = Some(c);
        true
    }

    /// Whether `c` is `scala.reflect.TypeTest`: the std's or a jar's, recognised by its path once.
    pub fn is_type_test_class(&mut self, c: ClassId) -> bool {
        if let Some(t) = self.sites.type_test {
            return t == c;
        }
        let name = self.syms.class(c).name;
        if self.name_str(name) != "TypeTest" || self.class_path(c) != "scala.reflect.TypeTest" {
            return false;
        }
        self.sites.type_test = Some(c);
        true
    }

    /// `TypeTest[S, T]` as scalac synthesizes it (dotty's second special handler,
    /// `typer/Synthesizer.scala` 95 to 126, `synthesizedTypeTest`): none for a `T` at the bottom
    /// (97); `TypeTest.identity[T]` where `S` conforms to `T`, which takes `null` (102 to 104);
    /// none for `AnyVal`, `AnyRef` and `Object` (105 to 106); otherwise an instance whose
    /// `unapply` is `(s: S) => if s.isInstanceOf[T] then Some(s.asInstanceOf[s.type & T]) else
    /// None` (108 to 124), the test being `T`'s as a pattern's, with its warning where `T` is
    /// abstract or its arguments cannot be checked. An anonymous class per use, as dotty's
    /// closure is one (`newAnonFun`, 121).
    pub fn type_test_given(&mut self, class: ClassId, target: TypeId, span: Span) -> Option<TExprId> {
        if !self.is_type_test_class(class) {
            return None;
        }
        // `fullyDefinedType` of both arguments (99 to 100).
        let target = self.solve_in(target);
        let Type::Class(_, args) = self.types.get(target) else { return None };
        let &[s, t] = self.types.items(args) else { return None };
        let tested = self.deref_alias(t);
        if matches!(self.types.get(tested), Type::Nothing) || self.class_of(tested) == Some(self.b.null) {
            return None;
        }
        let mark = self.snapshot();
        let conforms = self.is_sub(s, t);
        self.rollback(mark);
        if conforms {
            return self.type_test_identity(class, t, target);
        }
        if let Type::Class(c, _) = self.types.get(tested) {
            if c == self.b.any_val || c == self.b.any_ref {
                return None;
            }
        }
        let (some, none) = (self.std_class("Some")?, self.scala_class_named("None")?);
        let option = self.b.option?;
        self.complete_class(some);
        let (te, _) = self.sam_instance_typed(target, 1, span, |typer, params| {
            let (x, x_ty) = params[0];
            let scrut = typer.prog.add(TExpr::Local(x));
            typer.prog.set_type(scrut, x_ty);
            let test = typer.test_for(t, x_ty, span, false);
            let cond = typer.prog.add(TExpr::TypeTest(scrut, test));
            typer.prog.set_type(cond, typer.b.t_boolean);
            let value = typer.prog.add(TExpr::Local(x));
            typer.prog.set_type(value, x_ty);
            let (cast, cast_ty) = typer.lower_cast(value, x_ty, t, false, false);
            let l = typer.prog.list(&[cast]);
            let yes = typer.prog.add(TExpr::New(some, l));
            let some_ty = typer.types.class(some, &[cast_ty]);
            typer.prog.set_type(yes, some_ty);
            let no = typer.prog.add(TExpr::Module(none));
            let none_ty = typer.types.class(none, &[]);
            typer.prog.set_type(no, none_ty);
            let e = typer.prog.add(TExpr::If(cond, yes, Some(no)));
            let opt = typer.types.class(option, &[t]);
            typer.prog.set_type(e, opt);
            e
        })?;
        Some(te)
    }

    /// `TypeTest.identity[T]`, the companion's method, typed as the `target` asked for.
    fn type_test_identity(&mut self, class: ClassId, t: TypeId, target: TypeId) -> Option<TExprId> {
        let module = self.syms.class(class).companion?;
        self.complete_class(module);
        let name = self.interner.intern("identity");
        let sym = *self.syms.class(module).members.get(&name)?;
        let sym = match self.syms.alternatives(sym) {
            Some(alts) => *alts.first()?,
            None => sym,
        };
        let recv = self.prog.add(TExpr::Module(module));
        let call = self.prog.add(TExpr::CallMethod(recv, sym, ListRef::EMPTY));
        let tparams = self.sig_of(sym).tparams.clone();
        if let [p] = tparams.as_slice() {
            self.note_type_args(call, &[(*p, t)]);
        }
        self.prog.set_type(call, target);
        Some(call)
    }

    /// `A <:< B` when `A` conforms to `B`, and `A =:= B` when the two are the same type: the
    /// identity function, which the search may leave type variables of `B` bound by.
    pub fn evidence_given(&mut self, class: ClassId, target: TypeId, span: Span) -> Option<TExprId> {
        let equal = Some(class) == self.b.eq_evidence;
        if !equal && Some(class) != self.b.sub_evidence {
            return None;
        }
        let target = self.zonk(target);
        let Type::Class(_, args) = self.types.get(target) else { return None };
        let &[from, to] = self.types.items(args) else { return None };
        let mark = self.snapshot();
        let holds = if equal { self.is_same(from, to) } else { self.is_sub(from, to) };
        if !holds {
            self.rollback(mark);
            return None;
        }
        let x = self.fresh_local("ev", from, span);
        let body = self.prog.add(TExpr::Local(x));
        let params = self.prog.syms(&[x]);
        Some(self.prog.add(TExpr::Lambda(params, body)))
    }

    /// Whether `t` is a `scala.reflect.ClassTag[_]`.
    pub fn is_class_tag(&mut self, t: TypeId) -> bool {
        let Type::Class(c, _) = self.types.get(t) else { return false };
        if self.sites.class_tag.is_none() {
            let info = self.syms.class(c);
            let in_reflect = matches!(info.owner, Owner::Package(p) if self.interner.get(self.syms.pkg(p).name) == "reflect"
                && self.syms.pkg(p).parent == Some(self.b.scala_pkg));
            if in_reflect && self.interner.get(info.name) == "ClassTag" {
                self.sites.class_tag = Some(c);
            }
        }
        self.sites.class_tag == Some(c)
    }

    /// Where the evidence of a `@jvmEvidence` definition is erased: nothing is evaluated for
    /// it and nothing passed, and the definition has no parameter for it.
    pub fn erases_evidence(&self) -> bool {
        !self.jvm
    }

    /// The `ClassTag` evidence of the `@jvmEvidence` definition `sym` where it is erased. It is
    /// no parameter there, and a call still has to find it, as scalac's call does: each tag
    /// is searched for as a using clause's argument is, or taken from the using clause
    /// written for it, and dropped from the call. The std and the bodies of jars are taken on
    /// trust. What it found, in the tags' order, which the pickle passes: none where it took a
    /// tag on trust or found none. Where an argument of the call has a type that holds the error
    /// type (`erroneous_arg`), a tag of `Nothing` for a variable of the call that nothing bound
    /// from below is what that argument left: it conforms to its parameter without binding the
    /// variable, where scalac's application of it has the error type and searches no tag. Its
    /// absence depends on that error and is not presented. A `Nothing` written, bound by an
    /// argument (`List.empty[Nothing]`) or of an argument's own type (`throw`), beside an error
    /// inside an argument whose type it leaves alone, is reported as scalac reports it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn erased_evidence(&mut self, sym: SymId, sig: &MethodSig, subst: &Subst, written: Option<super::apply::ArgList>, erroneous_arg: bool, site: Span, span: Span) -> Vec<TExprId> {
        let tags = self.syms.sym(sym).erased_tags;
        let Some(tag_class) = self.sites.class_tag.filter(|_| tags != 0) else { return Vec::new() };
        let params: Vec<ParamSig> = sig
            .tparams
            .iter()
            .enumerate()
            .filter(|&(i, _)| i < 32 && tags & (1 << i) != 0)
            .map(|(_, &tp)| {
                let of = self.types.param(tp);
                ParamSig { name: names::EMPTY, ty: self.types.class(tag_class, &[of]), by_name: false, repeated: false, has_default: false, sym }
            })
            .collect();
        let count = params.len();
        if let Some(list) = written {
            let clause = ClauseSig { params, is_using: true, is_implicit: false };
            let mut typed = Vec::new();
            self.type_clause_args(&clause, list, subst, &mut typed);
            return if typed.len() == count { typed } else { Vec::new() };
        }
        let file = self.env.file;
        if self.source(file).is_std || self.in_jar(file) || self.is_body_file(file) {
            return Vec::new();
        }
        let determined = self.vars_of_applied_clauses(&sig.clauses, subst);
        let mut found = Vec::with_capacity(count);
        for p in params {
            let target = self.param_type(p.ty, subst);
            let tagged = self.deref(self.tagged(target));
            let unbound = erroneous_arg
                && match self.types.get(tagged) {
                    Type::Var(v) => self.tvars[v].inst.is_none() && self.tvars[v].lower.is_empty(),
                    _ => false,
                };
            self.solve_selected_in(target, &determined);
            let open = self.open_tag(target);
            if let Some((te, _)) = self.resolve_given_typed(target, site) {
                found.push(te);
                continue;
            }
            let missing = self.missing_tag(p.ty, target, open);
            let ambiguity = self.given_ambiguity.take();
            let after_failure = ambiguity.is_none() && unbound && {
                let solved = self.solve_bounded_in(target);
                self.deref(self.tagged(solved)) == NOTHING
            };
            if let Some(msg) = ambiguity.or(missing) {
                self.failed_givens.clear();
                if after_failure {
                    self.dependent_error(span, msg);
                } else {
                    self.error(span, msg);
                }
            }
        }
        if found.len() == count { found } else { Vec::new() }
    }

    /// Whether `target` is the type of a tag of a type that is open yet, which a search for
    /// the tag may settle.
    pub(super) fn open_tag(&mut self, target: TypeId) -> bool {
        if !self.is_class_tag(target) {
            return false;
        }
        let of = self.deref(self.tagged(target));
        matches!(self.types.get(of), Type::Var(_))
    }

    /// scalac's message for the tag of type `target` that a search did not find, declared as
    /// `declared`: it names the type, and the parameter where nothing but its bounds said
    /// what it is before the search (`B` of `xs.toArray`). `None` for what is no tag.
    pub(super) fn missing_tag(&mut self, declared: TypeId, target: TypeId, open: bool) -> Option<String> {
        if !self.is_class_tag(target) {
            return None;
        }
        let of = if open {
            self.tagged(declared)
        } else {
            let target = self.solve_bounded_in(target);
            self.tagged(target)
        };
        Some(format!("No ClassTag available for {}", self.show(of)))
    }

    fn tagged(&self, tag: TypeId) -> TypeId {
        match self.types.get(tag) {
            Type::Class(_, args) => self.types.items(args).first().copied().unwrap_or(tag),
            _ => tag,
        }
    }

    /// `ClassTag[T]` as scalac synthesizes it: the tag of `classOf` of the erasure of `T`, which
    /// has to be one that the type alone says (`stable_erasure`), and for an array type the
    /// element's tag, wrapped.
    pub fn class_tag_given(&mut self, class: ClassId, target: TypeId) -> Option<TExprId> {
        if Some(class) != self.sites.class_tag {
            return None;
        }
        let target = self.solve_bounded_in(target);
        let Type::Class(_, args) = self.types.get(target) else { return None };
        let subject = self.dealias(*self.types.items(args).first()?);
        let subject = self.deref(subject);
        let c = match self.types.get(subject) {
            // scalac has no tag of `Null`, as it has none of `Nothing`.
            Type::Class(c, _) if c == self.b.null => return None,
            Type::Class(c, _) => c,
            Type::Any => self.b.any_ref,
            _ => {
                let erased = self.stable_erasure(subject)?;
                return self.tag_of_erased(class, erased, subject);
            }
        };
        // `ClassTag[Array[E]]` is the element's tag's `wrap`, the JVM array class of `E`. The
        // element's tag is asked for on every target: where an array has no kind, the array's
        // tag is that of the one array class, and a program without the element's is
        // rejected all the same.
        if c == self.b.array {
            let Type::Class(_, elem) = self.types.get(subject) else { return None };
            let elem = *self.types.items(elem).first()?;
            let elem_tag = self.types.class(class, &[elem]);
            let evidence = self.resolve_given(elem_tag, crate::source::Span::default())?;
            if self.jvm {
                return self.wrapped_tag(class, evidence);
            }
        }
        self.tag_of_class(class, c, matches!(self.types.get(subject), Type::Any), subject)
    }

    /// `ClassTag[AnyRef]`, the tag of `java.lang.Object`.
    pub(super) fn any_ref_class_tag(&mut self, class: ClassId) -> Option<TExprId> {
        let any_ref = self.b.any_ref;
        let tagged = self.types.class(any_ref, &[]);
        self.tag_of_class(class, any_ref, false, tagged)
    }

    fn wrapped_tag(&mut self, class: ClassId, element: TExprId) -> Option<TExprId> {
        let wrap = self.interner.intern("wrap");
        let sym = *self.syms.class(class).members.get(&wrap)?;
        let none = self.prog.list(&[]);
        let call = super::apply::MethodCall { recv: Some(element), sym, owner_subst: Vec::new(), ext_recv: None, prefix: None };
        Some(self.build_call(&call, none))
    }

    /// The tag of `c`, a `ClassTag[tagged]`.
    fn tag_of_class(&mut self, class: ClassId, c: ClassId, any: bool, tagged: TypeId) -> Option<TExprId> {
        if self.link_mode() {
            return self.linked_class_tag(class, c, any, tagged);
        }
        let c = self.runtime_tuple_class(c);
        let of = self.prog.add(TExpr::ClassOf(c));
        let l = self.prog.list(&[of]);
        Some(self.prog.add(TExpr::New(class, l)))
    }

    /// The tag of an erasure: of its class, wrapped once per dimension on the JVM, and of the
    /// one array class elsewhere. `Nothing` and `Null` have no tag, as under scalac.
    fn tag_of_erased(&mut self, class: ClassId, erased: Erased, tagged: TypeId) -> Option<TExprId> {
        match erased {
            Erased::Nothing(_) | Erased::Null(_) => None,
            Erased::Of(c, 0) => self.tag_of_class(class, c, false, tagged),
            Erased::Of(c, dims) if self.jvm => {
                let array = self.dealias(tagged);
                let element = match self.types.get(array) {
                    Type::Class(a, args) if a == self.b.array => self.types.items(args).first().copied().unwrap_or(tagged),
                    _ => tagged,
                };
                let element = self.tag_of_erased(class, Erased::Of(c, dims - 1), element)?;
                self.wrapped_tag(class, element)
            }
            Erased::Of(..) => self.tag_of_class(class, self.b.array, false, tagged),
        }
    }

    /// scalac's erasure of a type whose erasure the type alone says (its `hasStableErasure`):
    /// a class type, a literal type, an array of such, a union or an intersection of such, and a
    /// proxy of such (a path, `C.this`, a refinement), which erases as what it stands for. A type
    /// parameter, an abstract type and an opaque type have none.
    pub(super) fn stable_erasure(&mut self, t: TypeId) -> Option<Erased> {
        // The cases `dealias` would see through to a bound: an abstract type and a type parameter
        // have no erasure of their own, a match type one only once it reduces (dotty's
        // `TypeBounds`, `TypeParamRef` and `MatchType` cases), and a path has its underlying
        // type's (the `TypeProxy` case).
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => return None,
            Type::Param(_) => {
                let known = self.dealias(t);
                return if known == t { None } else { self.stable_erasure(known) };
            }
            Type::Match(..) | Type::Alias(..) => {
                return match self.reduce_head(t) {
                    Some(r) if r != t => self.stable_erasure(r),
                    _ => None,
                };
            }
            Type::This(_) | Type::Term(_) | Type::Select(..) => {
                let widened = self.widen_path(t);
                return if widened == t { None } else { self.stable_erasure(widened) };
            }
            _ => {}
        }
        let t = self.dealias(t);
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Any => Some(Erased::Of(self.b.any_ref, 0)),
            Type::Nothing => Some(Erased::Nothing(0)),
            Type::Class(c, _) if c == self.b.null => Some(Erased::Null(0)),
            Type::Class(c, args) if c == self.b.array => {
                let element = self.stable_erasure(*self.types.items(args).first()?)?;
                Some(element.with_dims(element.dims() + 1))
            }
            // An opaque type erases to its underlying type, which the erasure alone sees.
            Type::Class(c, args) if self.syms.class(c).kind == ClassKind::Opaque => {
                self.complete_class(c);
                let info = self.syms.class(c);
                let under = info.underlying?;
                let subst: Subst = info.tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
                let under = self.types.subst(under, &subst);
                self.stable_erasure(under)
            }
            Type::Class(c, _) if c == self.b.any_val => Some(Erased::Of(self.b.any_ref, 0)),
            Type::Class(c, _) => Some(Erased::Of(c, 0)),
            Type::Lit(_) => match self.types.get(self.widen_lit(t)) {
                Type::Class(c, _) => Some(Erased::Of(c, 0)),
                _ => None,
            },
            Type::Union(a, b) => {
                let (a, b) = (self.stable_erasure(a)?, self.stable_erasure(b)?);
                Some(self.erased_lub(a, b))
            }
            Type::Inter(a, b) => {
                let (a, b) = (self.stable_erasure(a)?, self.stable_erasure(b)?);
                Some(if self.compare_erased(a, b) != std::cmp::Ordering::Greater { a } else { b })
            }
            Type::Refined(parent, _) => self.stable_erasure(parent),
            _ => None,
        }
    }

    /// The classes whose values are no references once erased; `Unit` is not one of them, its
    /// erasure being `BoxedUnit` wherever a value of it is held.
    fn is_primitive_class(&self, c: ClassId) -> bool {
        let b = &self.b;
        [b.int, b.long, b.double, b.byte, b.short, b.float, b.boolean, b.char].contains(&c)
    }

    /// scalac's `erasedLub`, the erasure of a union. `Nothing` gives way to the other side and
    /// `Null` to a reference. Two arrays of references are an array of the elements' lub, two of
    /// one primitive are that array, and any other pair with an array in it is `Object`. Two
    /// classes are a class or a trait both derive from: of those in the first one's
    /// linearisation up to the first trait, the last that none of the others derives from.
    pub(super) fn erased_lub(&mut self, a: Erased, b: Erased) -> Erased {
        let object = Erased::Of(self.b.any_ref, 0);
        let reference = |s: &Self, e: Erased| match e {
            Erased::Of(c, dims) => dims > 0 || !s.is_primitive_class(c),
            Erased::Nothing(dims) | Erased::Null(dims) => dims > 0,
        };
        match (a, b) {
            (Erased::Nothing(0), x) | (x, Erased::Nothing(0)) => x,
            (Erased::Null(0), Erased::Null(0)) => a,
            (Erased::Null(0), x) | (x, Erased::Null(0)) => if reference(self, x) { x } else { object },
            _ if a.dims() > 0 && b.dims() > 0 => {
                let (e1, e2) = (a.element(), b.element());
                if e1 == e2 {
                    return a;
                }
                // A primitive element meets no other element in an array.
                let bottom = |e: Erased| matches!(e, Erased::Nothing(0) | Erased::Null(0));
                if (!reference(self, e1) && !bottom(e1)) || (!reference(self, e2) && !bottom(e2)) {
                    return object;
                }
                let lub = self.erased_lub(e1, e2);
                if !reference(self, lub) && !bottom(lub) {
                    return object;
                }
                lub.with_dims(lub.dims() + 1)
            }
            _ if a.dims() > 0 || b.dims() > 0 => object,
            (Erased::Nothing(_), _) | (_, Erased::Nothing(_)) | (Erased::Null(_), _) | (_, Erased::Null(_)) => object,
            (Erased::Of(c1, _), Erased::Of(c2, _)) if c1 == c2 => a,
            (Erased::Of(c1, _), Erased::Of(c2, _)) => {
                self.complete_class(c1);
                let bases: Vec<ClassId> = self.syms.class(c1).base_types.iter().map(|&(b, _)| b).collect();
                let mut candidates = Vec::new();
                for base in bases {
                    if !self.derives_from(c2, base) {
                        continue;
                    }
                    candidates.push(base);
                    if self.syms.class(base).kind == ClassKind::Trait {
                        break;
                    }
                }
                let minimal = |s: &mut Self, cand: ClassId| candidates.iter().all(|&x| x == cand || !s.derives_from(x, cand));
                let lub = candidates.iter().rev().copied().find(|&cand| minimal(self, cand));
                match lub {
                    Some(c) if c != self.b.any_val => Erased::Of(c, 0),
                    _ => object,
                }
            }
        }
    }

    /// The order of scalac's `compareErasedGlb`, whose smaller side is the erasure of an
    /// intersection: a value class before any other type, an array before what is none (two
    /// arrays by their elements), a primitive before a reference, a class before a trait, and
    /// among two of a kind the one that derives from the other, or else the one whose full
    /// name comes first.
    pub(super) fn compare_erased(&mut self, a: Erased, b: Erased) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        let (c1, d1, c2, d2) = match (a, b) {
            (Erased::Of(c1, d1), Erased::Of(c2, d2)) => (c1, d1, c2, d2),
            (Erased::Of(..), _) => return Greater,
            (_, Erased::Of(..)) => return Less,
            (Erased::Nothing(_), _) => return Less,
            _ => return Greater,
        };
        if a == b {
            return Equal;
        }
        let first = |x: bool, y: bool| match (x, y) {
            (true, false) => Some(Less),
            (false, true) => Some(Greater),
            _ => None,
        };
        let value = |s: &Self, c: ClassId, dims: u8| dims == 0 && s.syms.class(c).value_class;
        if let Some(order) = first(value(self, c1, d1), value(self, c2, d2)) {
            return order;
        }
        if d1 > 0 && d2 > 0 {
            return self.compare_erased(Erased::Of(c1, d1 - 1), Erased::Of(c2, d2 - 1));
        }
        if let Some(order) = first(d1 > 0, d2 > 0) {
            return order;
        }
        if let Some(order) = first(self.is_primitive_class(c1), self.is_primitive_class(c2)) {
            return order;
        }
        let real = |s: &Self, c: ClassId| s.syms.class(c).kind != ClassKind::Trait;
        if let Some(order) = first(real(self, c1), real(self, c2)) {
            return order;
        }
        if self.derives_from(c1, c2) {
            Less
        } else if self.derives_from(c2, c1) {
            Greater
        } else {
            crate::text::utf16_units(&self.class_path(c1)).cmp(crate::text::utf16_units(&self.class_path(c2)))
        }
    }

    /// In link mode the tag is scala-library's, an interface whose instances `ClassTag$` makes:
    /// `ClassTag.apply(classOf[T])`, and `ClassTag.Any` for `Any`, as scalac's.
    fn linked_class_tag(&mut self, class: ClassId, c: ClassId, any: bool, tagged: TypeId) -> Option<TExprId> {
        let module = self.syms.class(class).companion?;
        self.complete_class(module);
        let member = self.interner.intern(if any { "Any" } else { "apply" });
        let sym = *self.syms.class(module).members.get(&member)?;
        let sym = match self.syms.alternatives(sym) {
            Some(alts) => *alts.first()?,
            None => sym,
        };
        let recv = self.prog.add(TExpr::Module(module));
        if any {
            return Some(self.prog.add(TExpr::Field(recv, sym)));
        }
        let of = self.prog.add(TExpr::ClassOf(c));
        let l = self.prog.list(&[of]);
        let call = self.prog.add(TExpr::CallMethod(recv, sym, l));
        // An inline body's record keeps the type arguments of each of its generic calls.
        let tparams = self.sig_of(sym).tparams.clone();
        if let [t] = tparams.as_slice() {
            self.note_type_args(call, &[(*t, tagged)]);
        }
        Some(call)
    }

    /// `ValueOf[L]` for a literal type `L`, as scalac synthesizes it.
    pub fn value_of_given(&mut self, class: ClassId, target: TypeId) -> Option<TExprId> {
        let info = self.syms.class(class);
        if info.owner != Owner::Package(self.b.scala_pkg) || self.interner.get(info.name) != "ValueOf" || Some(class) != self.value_of_class() {
            return None;
        }
        let target = self.zonk(target);
        let Type::Class(_, args) = self.types.get(target) else { return None };
        let &[arg] = self.types.items(args) else { return None };
        let mut arg = self.deref(arg);
        // A given's type variable is still open while its using clause is resolved; what the
        // target fixed it to is what the value is made from.
        if let Type::Var(v) = self.types.get(arg) {
            if !self.tvars[v].lower.is_empty() || !self.tvars[v].upper.is_empty() {
                self.solve_var(v);
                arg = self.deref(arg);
            }
        }
        let Type::Lit(l) = self.types.get(arg) else { return None };
        let value = self.literal_expr(self.types.lit_val(l));
        let l = self.prog.list(&[value]);
        Some(self.prog.add(TExpr::New(class, l)))
    }
}
