//! The `@jvm` templates as nodes of the lowered IR (`crate::lir`). A template's text is read
//! once; a use whose text is a tree becomes nodes over its operands, and every other use is
//! assembled from the text (`Gen::assemble`). Of the texts the typer makes itself, one that
//! is a call becomes that call; `Gen::intrinsic` writes the others.

use crate::lir::{Arith, Cast, Comparing, ErasedType, Invoke, Kind, LirId, MethodRef, Refusal, Relation};
use super::classfile::op;
use super::gen::{erased_array_method, primitive_conversion, Gen, Referenced, Site};
use super::intrinsic::{plain_op, single_form, split_method, tokenize, typer_text};
use super::names::*;
use super::write::arith_instruction;
use crate::ast::ListRef;
use crate::symbols::Owner;
use crate::tir::*;
use crate::types::{ClassId, Type};
use crate::typer::prims::UNBOX;
use std::rc::Rc;

/// What a template's text is, whatever uses it.
pub(super) enum Template {
    /// One member reference and nothing else (`single_form`): its operands are the last of the
    /// arguments, as many as the reference takes.
    Single(Rc<MethodRef>),
    /// Stack code that is a tree.
    Tree(Rc<Tree>),
    /// Stack code no tree is read from: the token of the instruction where reading stopped.
    Unread(usize),
}

/// Stack code in which every value is left once and taken once, by the instruction that
/// follows its operands: the instructions in the text's order are a tree's nodes, operands
/// first.
pub(super) struct Tree {
    ops: Vec<Op>,
    /// How many arguments the placeholders reach.
    arguments: usize,
    /// The token each op was read from, which a refusal names (`Gen::template_route`).
    at: Vec<usize>,
    /// It makes an array of the use's type (`array_new`), which must be an array's.
    makes_array: bool,
}

enum Op {
    /// `$n`, `$n:T`: an argument, at the type its expression has or at `T`.
    Operand(usize, Option<ErasedType>),
    /// `invokevirtual`, `invokeinterface`, `rtcall`: the method of the value under its
    /// arguments, called as it is spelled.
    Call(Rc<MethodRef>),
    /// `invokestatic`.
    StaticCall(Rc<MethodRef>),
    /// `new C dup`, the arguments, `invokespecial C.<init>`: the constructor.
    New(Rc<MethodRef>),
    /// `getstatic`: the class, the field, its type.
    StaticField(Rc<str>, Rc<str>, ErasedType),
    /// `getfield`: the field of the instance under it: the class, the field, its type.
    Field(Rc<str>, Rc<str>, ErasedType),
    /// `putfield`: the field of the instance under the value set to it; no value.
    FieldStore(Rc<str>, Rc<str>, ErasedType),
    /// `putstatic`: the static field set to the value under it; no value.
    StaticFieldStore(Rc<str>, Rc<str>, ErasedType),
    /// `ldc "text"`: the string constant.
    Text(Rc<str>),
    /// `rt`, `getstatic C.MODULE$:LC;`: the instance of the object whose class is `C`.
    Module(Rc<str>),
    /// `ipush n`, `iconst_n`.
    Int(i32),
    /// `aconst_null`: the null reference.
    Null,
    /// A primitive conversion, to this type, by this instruction.
    Convert(ErasedType, u8),
    /// An operation on two values of this type.
    Binary(Arith, ErasedType),
    /// `iconst_1 ixor` on a boolean: its negation.
    Not,
    /// `ineg`, `lneg`, `dneg`: the number negated, of this type.
    Neg(ErasedType),
    /// `lcmp`, `dcmpl`, `dcmpg`: the `Int` -1, 0 or 1 by how the two values of this type compare; a
    /// NaN on either side gives 1 where the flag says so (`dcmpg`), else -1 (`dcmpl`).
    ThreeWay(ErasedType, bool),
    /// `cast_result`: the value as the type the typer recorded for the use.
    AsRecorded,
    /// `checkcast C`: the reference tested to be null or an instance of the class or array
    /// class `C`.
    Cast(Rc<str>),
    /// `instanceof C`: whether the reference is an instance of the class or array class `C`.
    Is(Rc<str>),
    /// A lone operand at a reference type: the argument as it is, the def a cast that erasure
    /// drops (`unsafeCast`).
    Erased,
    /// `array_apply`: an element of the array under the index read.
    ArrayLoad,
    /// `array_update`: an element of the array under the index and the value assigned.
    ArrayStore,
    /// `array_length`.
    ArrayLength,
    /// `array_clone`.
    ArrayClone,
    /// `array_new`: a new array of the use's type, of the length under it.
    NewArray,
    /// `builder_append`, `builder_insert`: `java.lang.StringBuilder`'s `append` (`insert` at the
    /// index under the value) whose parameter the value's own type picks (`Gen::rendered_as`).
    BuilderAppend,
    BuilderInsert,
    /// A last `pop`: the value dropped.
    Drop,
    /// `dup`: the value held in a temporary, read by each of its two copies.
    Dup,
    /// A `pop` with a value under it: the value dropped, a statement of the template's block.
    Statement,
    /// `athrow`.
    Throw,
}

/// The way a use of a template is generated: one decision (`Gen::template_route`), read by the
/// lowering and by `Gen::intrinsic`, which holds the routes not lowered yet.
pub(super) enum TemplateRoute {
    /// The call of one member over the last of the arguments.
    Single(Rc<MethodRef>),
    /// The tree its stack code is, over the arguments it names.
    Tree(Rc<Tree>),
    /// The typer's text for a member of `Object`: called on the first argument, named on the
    /// class that argument points at (`Gen::object_member_site`), over the next ones.
    ObjectMember(Rc<MethodRef>),
    /// The typer's text for a member it names: called on the first argument at the member's
    /// class, over the next ones; with no receiver, over the first ones.
    Member(Rc<MethodRef>),
    /// The typer's text for an instance of a class of the runtime: its constructor over the
    /// arguments.
    Made(Rc<MethodRef>),
    /// The typer's text for `x.synchronized(body)`, which it keeps for the JVM: the body under the
    /// monitor of the first argument.
    Synchronized,
    /// The typer's text `($0 ?? $1)`: the first argument unboxed to the primitive the second is
    /// of, the second where the first is null (`Gen::lower_unbox`).
    Unbox,
    /// A quote, or a member of the std's quoted reflection, which runs at compile time only: a
    /// method holding one that the output keeps (an override of a jar trait's member,
    /// `Literally.validate`, `TreeMethods.asExpr` beside a macro's expansion) throws if it is
    /// ever called.
    CompileTimeOnly,
    /// A template the JVM has no translation of, refused: a member with no `@jvm` template or
    /// body, or a JavaScript text of the typer's.
    Refused(Refusal),
}

impl<'a> Gen<'a> {
    fn template(&mut self, text: &str) -> Rc<Template> {
        if let Some(read) = self.templates.get(text) {
            return read.clone();
        }
        let read = Rc::new(read_template(text));
        self.templates.insert(Box::from(text), read.clone());
        read
    }

    /// How the use `e` of a template is generated; a new array of a type that is no array's is
    /// refused by `Gen::new_array`.
    pub(super) fn template_route(&mut self, e: TExprId, template: StrRef, args: &[TExprId]) -> TemplateRoute {
        let cx = self.cx;
        let text: &str = &cx.input.prog.strings[template.idx()];
        if text.starts_with('!') || typer_text(text) {
            if let Some(route) = self.typer_route(template, text, args) {
                return route;
            }
            if text.starts_with("$quote") || text.starts_with('!') && self.template_owner(template).is_some_and(|o| o.starts_with("scala/quoted/")) {
                return TemplateRoute::CompileTimeOnly;
            }
            return TemplateRoute::Refused(self.refused_template(template, text));
        }
        let tokens = || tokenize(text);
        let refused = |what: &'static str, item: &str| TemplateRoute::Refused(Refusal::Template { what, item: Rc::from(item), text: Rc::from(text) });
        match &*self.template(text) {
            Template::Single(m) if args.len() >= m.params.len() + (m.kind != Invoke::Static) as usize => TemplateRoute::Single(m.clone()),
            Template::Single(_) => refused("instruction", &tokens()[0]),
            Template::Tree(tree) if args.len() < tree.arguments => {
                let tokens = tokens();
                let beyond = tokens.iter().find(|t| t.strip_prefix('$').and_then(|r| r.split(':').next()?.parse::<usize>().ok()).is_some_and(|n| n >= args.len()));
                refused("placeholder", beyond.map_or("", |t| t.as_str()))
            }
            Template::Tree(tree) if tree.makes_array && Self::array_element(&self.recorded_erasure(e)).is_none() => TemplateRoute::Refused(Refusal::NewArrayOfUnknownElement),
            Template::Tree(tree) => match self.tree_unfit(tree, args) {
                None => TemplateRoute::Tree(tree.clone()),
                Some(k) => refused("instruction", &tokens()[tree.at[k]]),
            },
            Template::Unread(at) => {
                let (what, item) = unread(&tokens(), *at);
                refused(what, &item)
            }
        }
    }

    /// The refusal of a template the JVM has no translation of: a member whose template names no
    /// body (`!name`, the std's quoted reflection aside, `TemplateRoute::CompileTimeOnly`); an
    /// enum's lookup whose companion is not known; a partial function where the runtime lacks its
    /// class; a typer's text that is JavaScript's alone. A primitive's class, a product's prefix and
    /// an unboxing with too few arguments (a program's own `@jvm` on a top-level def) are refused
    /// like the others.
    fn refused_template(&mut self, template: StrRef, text: &str) -> Refusal {
        if let Some(name) = text.strip_prefix('!') {
            let owner = self.template_owner(template);
            return Refusal::NoJvmBody { name: Rc::from(name), owner };
        }
        match text {
            "$enumValues($0)" | "$enumValueOf($0, $1, $2, $3, $4)" | "$enumFromOrdinal($0, $1, $2)" | "$enumCaseOfOrdinal($0, $1, $2, $3, $4)" => Refusal::EnumLookupWithoutCompanion,
            "$pf($0, $1)" => Refusal::PartialFunctionWithoutRuntimeClass,
            _ => Refusal::JavaScriptTemplate(Rc::from(text)),
        }
    }

    /// A use of a template: its nodes, or its refusal. A member's template reached through `super`
    /// is the member's super call, as scalac writes it (`invokespecial` on the class `super` names,
    /// `Gen::lower_super_call`): the template stands for the member called on an instance, which a
    /// super call bypasses; a member of the program that carries its own template is refused there.
    pub(super) fn lower_template(&mut self, e: TExprId, template: StrRef, list: ListRef) -> LirId {
        let cx = self.cx;
        let args: &[TExprId] = cx.input.prog.expr_list(list);
        let text: &str = &cx.input.prog.strings[template.idx()];
        if let (Some(&r), Some((name, ret))) = (args.first(), any_member_over_super(text)) {
            if matches!(cx.input.prog.expr(r), TExpr::Super(_)) {
                return self.lower_object_super_call(e, name, Vec::new(), ret, Vec::new());
            }
        }
        if let (Some(&s), Some(&r)) = (cx.input.prog.template_syms.get(&template), args.first()) {
            if matches!(cx.input.prog.expr(r), TExpr::Super(_)) {
                let std = cx.input.sources.get(cx.input.syms.sym(s).file.0 as usize).is_some_and(|f| f.is_std);
                if !std {
                    let member = Rc::from(cx.input.interner.get(cx.input.syms.sym(s).name));
                    return self.lower_refused(Refusal::SuperCallOfOwnTemplate { member });
                }
                return self.lower_super_call(e, r, s, ListRef { start: list.start + 1, len: list.len - 1 });
            }
        }
        match self.template_route(e, template, args) {
            TemplateRoute::Single(m) => self.lower_single(args, &m),
            TemplateRoute::Tree(tree) => self.lower_tree(e, args, &tree),
            TemplateRoute::ObjectMember(m) => self.lower_object_member(args, &m),
            TemplateRoute::Member(m) => self.lower_member(args, &m),
            TemplateRoute::Made(ctor) => {
                let mut operands: Vec<LirId> = Vec::with_capacity(ctor.params.len());
                for (a, p) in args.iter().zip(&ctor.params) {
                    operands.push(self.lower_expr(*a, p));
                }
                let operands = self.lir.list(&operands);
                self.lir.push(Kind::New { touch: None, ctor: ctor.clone(), args: operands }, ErasedType::L(ctor.owner.clone()))
            }
            TemplateRoute::Synchronized => self.lower_synchronized(args[0], args[1]),
            TemplateRoute::Unbox => self.lower_unbox(args[0], args[1]),
            TemplateRoute::CompileTimeOnly => {
                let exception: Rc<str> = Rc::from("java/lang/UnsupportedOperationException");
                let string = ErasedType::L(Rc::from(STRING));
                let ctor = Rc::new(reference(Invoke::Special, &exception, "<init>", vec![string.clone()], ErasedType::V));
                let message = self.lir.push(Kind::Str(Rc::from("runs at compile time only")), string);
                let args = self.lir.list(&[message]);
                let made = self.lir.push(Kind::New { touch: None, ctor, args }, ErasedType::L(exception));
                self.lir.push(Kind::Throw(made), ErasedType::V)
            }
            TemplateRoute::Refused(refusal) => self.lower_refused(refusal),
        }
    }

    /// The typer's own texts that are one call; None for the others, which `Gen::intrinsic`
    /// writes.
    fn typer_route(&mut self, template: StrRef, text: &str, args: &[TExprId]) -> Option<TemplateRoute> {
        let object = ErasedType::object();
        let named = |kind: Invoke, owner: &str, name: &str, params: Vec<ErasedType>, ret: ErasedType| Rc::new(reference(kind, owner, name, params, ret));
        if let Some(name) = text.strip_prefix('!') {
            // `scala.reflect.Enum.ordinal`, the std's trait standing for the jar's interface, is
            // that interface's method (a macro's `e.ordinal` on an `E <: Enum`).
            let owner = self.template_owner(template)?;
            return (name == "ordinal" && args.len() == 1 && &*owner == ENUM).then(|| TemplateRoute::Member(named(Invoke::Interface, ENUM, "ordinal", Vec::new(), ErasedType::I)));
        }
        Some(match text {
            "$hashCode($0)" if !args.is_empty() => TemplateRoute::ObjectMember(named(Invoke::Virtual, OBJECT, "hashCode", Vec::new(), ErasedType::I)),
            "$equals($0, $1)" if args.len() >= 2 => TemplateRoute::ObjectMember(named(Invoke::Virtual, OBJECT, "equals", vec![object], ErasedType::Z)),
            "$getClass($0)" if !args.is_empty() => TemplateRoute::ObjectMember(named(Invoke::Virtual, OBJECT, "getClass", Vec::new(), ErasedType::L(Rc::from("java/lang/Class")))),
            "$productPrefix($0)" if !args.is_empty() => TemplateRoute::Member(named(Invoke::Interface, PRODUCT, "productPrefix", Vec::new(), ErasedType::L(Rc::from(STRING)))),
            "$synchronized($0, $1)" if args.len() >= 2 => TemplateRoute::Synchronized,
            UNBOX if args.len() >= 2 => TemplateRoute::Unbox,
            "$identityHash($0)" if !args.is_empty() => TemplateRoute::Member(named(Invoke::Static, "java/lang/System", "identityHashCode", vec![object], ErasedType::I)),
            "$0.$ordinal" if !args.is_empty() => TemplateRoute::Member(named(Invoke::Interface, ENUM, "ordinal", Vec::new(), ErasedType::I)),
            // The lookups of an enum's companion (`prims.rs`): its own `values`, `valueOf` and
            // `fromOrdinal`, scalac's (`enum_companion_members`), over the arguments they take.
            "$enumValues($0)" | "$enumValueOf($0, $1, $2, $3, $4)" | "$enumFromOrdinal($0, $1, $2)" | "$enumCaseOfOrdinal($0, $1, $2, $3, $4)" => {
                let (companion, of) = self.enum_lookup(args)?;
                let owner = self.class_name(companion);
                let enum_type = self.class_type(of);
                let lookup = match text {
                    "$enumValues($0)" => {
                        let array = ErasedType::L(Rc::from(format!("[{}", self.desc_of(&enum_type))));
                        named(Invoke::Virtual, &*owner, "values", Vec::new(), array)
                    }
                    "$enumValueOf($0, $1, $2, $3, $4)" => named(Invoke::Virtual, &*owner, "valueOf", vec![ErasedType::L(Rc::from(STRING))], enum_type),
                    _ => named(Invoke::Virtual, &*owner, "fromOrdinal", vec![ErasedType::I], enum_type),
                };
                if args.len() <= lookup.params.len() {
                    return None;
                }
                TemplateRoute::Member(lookup)
            }
            "$pf($0, $1)" => {
                let &c = self.cx.helpers.classes.get("PartialFunctionImpl")?;
                let owner = self.class_name(c);
                let params = self.ctor_param_types(c);
                TemplateRoute::Made(Rc::new(reference(Invoke::Special, &*owner, "<init>", params, ErasedType::V)))
            }
            _ => return None,
        })
    }

    /// `super.name(args)` of a member of `Object` that no class behind the caller defines
    /// (`Typer::apply_super_any_member`), as scalac writes it: `invokespecial` on the superclass,
    /// which names it (`Gen::lower_super_call`'s owner).
    pub(super) fn lower_object_super_call(&mut self, e: TExprId, name: &str, params: Vec<ErasedType>, ret: ErasedType, args: Vec<LirId>) -> LirId {
        let cx = self.cx;
        let owner = match self.this_class.and_then(|c| cx.input.syms.class(c).superclass) {
            Some(c) => self.class_name(c),
            None => Rc::from(OBJECT),
        };
        let desc = method_desc(&params, &ret);
        let method = Rc::new(MethodRef { owner, owner_is_interface: false, name: name.to_string(), params, ret: ret.clone(), desc, kind: Invoke::Special });
        let receiver = self.lower_instance();
        let args = self.lir.list(&args);
        let place = cx.input.prog.span_of(e).map(|(file, span)| (file, span.end));
        self.lir.push(Kind::SuperCall { method, receiver, args, order: None, place }, ret)
    }

    /// `($0 ?? $1)`, teq's text for a value of a reflective call unboxed to a primitive
    /// (`typer::prims::UNBOX`): the value held, its unboxing where it is not null, else `zero`, at
    /// the primitive's type, which a later ascription of the value leaves alone.
    fn lower_unbox(&mut self, value: TExprId, zero: TExprId) -> LirId {
        let object = ErasedType::object();
        let prim = self.recorded_erasure(zero);
        let held = self.lir.temp();
        let init = self.lower_expr(value, &object);
        let keep = self.lir.keep(held, init);
        let read = self.lir.kept(held, object.clone());
        let null = self.lir.push(Kind::Null, object.clone());
        let present = self.lir.push(Kind::Compare { relation: Relation::Ne, how: Comparing::Identity, left: read, right: null }, ErasedType::Z);
        let boxed = self.lir.kept(held, object.clone());
        let unboxed = self.lower_adapt(boxed, &object, &prim);
        let otherwise = self.lower_expr(zero, &prim);
        let value = self.lir.push(Kind::If { cond: present, then: unboxed, otherwise: Some(otherwise) }, prim);
        self.lir.seq(&[keep], value)
    }

    /// The name of the class that declares the def a template stands on.
    pub(super) fn template_owner(&mut self, template: StrRef) -> Option<Rc<str>> {
        let cx = self.cx;
        match cx.input.syms.sym(*cx.input.prog.template_syms.get(&template)?).owner {
            Owner::Class(c) => Some(self.class_name(c)),
            _ => None,
        }
    }

    /// The companion an enum's lookup is made on, the first argument, and its enum.
    fn enum_lookup(&self, args: &[TExprId]) -> Option<(ClassId, ClassId)> {
        let cx = self.cx;
        let prog = cx.input.prog;
        let first = *args.first()?;
        let module = |e: TExprId| match prog.expr(e) {
            TExpr::Module(c) => Some(c),
            _ => None,
        };
        let held = match prog.expr(first) {
            TExpr::Block(_, res) => module(res),
            _ => module(first),
        };
        let companion = held.or_else(|| match cx.input.types.get(self.recorded_type(first)?) {
            Type::Class(c, _) => Some(c),
            _ => None,
        })?;
        Some((companion, cx.input.syms.class(companion).companion?))
    }

    /// `Object`'s member called on the first argument, named on the class it points at.
    fn lower_object_member(&mut self, args: &[TExprId], m: &Rc<MethodRef>) -> LirId {
        let (site, qualifier) = self.object_member_site(args[0], &m.name);
        let receiver = self.lower_expr(args[0], &qualifier);
        let mut operands: Vec<LirId> = Vec::with_capacity(m.params.len());
        for (a, p) in args[1..].iter().zip(&m.params) {
            operands.push(self.lower_expr(*a, p));
        }
        let operands = self.lir.list(&operands);
        self.lir.push(Kind::Call { method: m.clone(), at: site.map(|site| (site.owner, site.interface)), receiver, args: operands, order: None }, m.ret.clone())
    }

    /// A member the typer's text names: the receiver at the member's class, each argument at the
    /// type the member takes.
    fn lower_member(&mut self, args: &[TExprId], m: &Rc<MethodRef>) -> LirId {
        let receiver = (m.kind != Invoke::Static).then(|| self.lower_expr(args[0], &ErasedType::L(m.owner.clone())));
        let mut operands: Vec<LirId> = Vec::with_capacity(m.params.len());
        for (a, p) in args[receiver.is_some() as usize..].iter().zip(&m.params) {
            operands.push(self.lower_expr(*a, p));
        }
        let operands = self.lir.list(&operands);
        let ret = m.ret.clone();
        match receiver {
            Some(receiver) => self.lir.push(Kind::Call { method: m.clone(), at: None, receiver, args: operands, order: None }, ret),
            None => self.lir.push(Kind::StaticCall { method: m.clone(), args: operands, order: None }, ret),
        }
    }

    /// The operands of a template that is one member reference: how many leading arguments it
    /// leaves out (the object of a stand-in's static is never evaluated), the type each of
    /// the others is evaluated at, and where a virtual call points: the JDK's member called on
    /// the receiver is named on the class the receiver points at, as the program's members are
    /// (`Gen::method_site`).
    pub(super) fn single_operands(&mut self, args: &[TExprId], kind: Invoke, owner: &Rc<str>, name: &str, desc: &str, params: &[ErasedType]) -> (usize, Vec<ErasedType>, Option<Site>) {
        let mut wanted: Vec<ErasedType> = Vec::new();
        if kind != Invoke::Static {
            wanted.push(ErasedType::L(owner.clone()));
        }
        wanted.extend(params.iter().cloned());
        let skip = args.len().saturating_sub(wanted.len());
        let mut site = None;
        if matches!(kind, Invoke::Virtual | Invoke::Interface) && skip < args.len() {
            site = self.method_site(args[skip], owner, kind == Invoke::Interface, name, desc, Referenced::Method(name));
            if let Some(site) = &site {
                wanted[0] = site.qualifier.clone();
            }
        }
        (skip, wanted, site)
    }

    /// The operands are at the types the reference spells, one by one: an array among them is
    /// the argument's own value, never a varargs literal's.
    fn lower_single(&mut self, args: &[TExprId], m: &Rc<MethodRef>) -> LirId {
        let (skip, wanted, site) = self.single_operands(args, m.kind, &m.owner, &m.name, &m.desc, &m.params);
        let mut operands: Vec<LirId> = Vec::with_capacity(wanted.len());
        for (a, w) in args[skip..].iter().zip(&wanted) {
            operands.push(self.lower_expr(*a, w));
        }
        let ret = m.ret.clone();
        if m.kind == Invoke::Static {
            let args = self.lir.list(&operands);
            return self.lir.push(Kind::StaticCall { method: m.clone(), args, order: None }, ret);
        }
        let args = self.lir.list(&operands[1..]);
        self.lir.push(Kind::Call { method: m.clone(), at: site.map(|site| (site.owner, site.interface)), receiver: operands[0], args, order: None }, ret)
    }

    /// The type an argument is evaluated at where its placeholder names none: its static
    /// type, a unit as the boxed one. A value class's instance is its box, the parameter being
    /// generic (the element `update` stores in a `V[]`): no std template declares one.
    pub(super) fn placeholder_type(&mut self, arg: TExprId) -> ErasedType {
        match self.expr_value_class(arg) {
            Some((c, _)) => self.class_type(c),
            None => match self.static_type(arg) {
                ErasedType::V => ErasedType::L(Rc::from(BOXED_UNIT)),
                t => t,
            },
        }
    }

    /// The nodes of a tree's instructions, each over the nodes the ones before it left. An
    /// operand of a call is at the type the template gives it, which the parameter takes and
    /// need not equal.
    fn lower_tree(&mut self, e: TExprId, args: &[TExprId], tree: &Tree) -> LirId {
        let mut stack: Vec<LirId> = Vec::new();
        // A `dup`'s temporary and the statements after it, which the template's value follows.
        let mut statements: Vec<LirId> = Vec::new();
        let take = |stack: &mut Vec<LirId>| stack.pop().expect("a tree, by `read_tree`");
        for op in &tree.ops {
            let node = match op {
                Op::Operand(n, declared) => {
                    let want = match declared {
                        Some(t) => t.clone(),
                        None => self.placeholder_type(args[*n]),
                    };
                    self.lower_expr(args[*n], &want)
                }
                Op::Call(m) => {
                    let operands = self.lir.list(&stack[stack.len() - m.params.len()..]);
                    stack.truncate(stack.len() - m.params.len());
                    let receiver = take(&mut stack);
                    self.lir.push(Kind::Call { method: m.clone(), at: None, receiver, args: operands, order: None }, m.ret.clone())
                }
                Op::StaticCall(m) => {
                    let operands = self.lir.list(&stack[stack.len() - m.params.len()..]);
                    stack.truncate(stack.len() - m.params.len());
                    self.lir.push(Kind::StaticCall { method: m.clone(), args: operands, order: None }, m.ret.clone())
                }
                Op::New(ctor) => {
                    let operands = self.lir.list(&stack[stack.len() - ctor.params.len()..]);
                    stack.truncate(stack.len() - ctor.params.len());
                    self.lir.push(Kind::New { touch: None, ctor: ctor.clone(), args: operands }, ErasedType::L(ctor.owner.clone()))
                }
                Op::StaticField(owner, name, ty) => self.lir.push(Kind::StaticField { owner: owner.clone(), name: name.clone() }, ty.clone()),
                Op::Field(owner, name, ty) => {
                    let receiver = take(&mut stack);
                    self.lir.push(Kind::Field { receiver, owner: owner.clone(), name: name.clone() }, ty.clone())
                }
                // A field set leaves no value: the template's value where it is the last
                // instruction, else a statement of its block (`Op::Statement`).
                Op::FieldStore(owner, name, ty) | Op::StaticFieldStore(owner, name, ty) => {
                    let instance = matches!(op, Op::FieldStore(..));
                    let value = take(&mut stack);
                    let held = self.lir.ty(value).clone();
                    let value = self.lower_adapt(value, &held, ty);
                    let (owner, name) = (owner.clone(), name.clone());
                    match instance {
                        true => {
                            let receiver = take(&mut stack);
                            self.lir.push(Kind::FieldStore { receiver, owner, name, value }, ErasedType::V)
                        }
                        false => self.lir.push(Kind::StaticFieldStore { owner, name, value }, ErasedType::V),
                    }
                }
                Op::Text(text) => self.lir.push(Kind::Str(text.clone()), ErasedType::L(Rc::from(STRING))),
                Op::Module(class) => self.lir.push(Kind::Module(class.clone()), ErasedType::L(class.clone())),
                Op::Int(v) => self.lir.push(Kind::Int(*v), ErasedType::I),
                Op::Null => self.lir.push(Kind::Null, ErasedType::object()),
                Op::Convert(to, _) => {
                    let value = take(&mut stack);
                    let from = self.lir.ty(value).clone();
                    self.lower_adapt(value, &from, to)
                }
                // An operand of a smaller integral type or a boolean is an `Int`'s (`Gen::tree_unfit`).
                Op::Binary(arith, ty) => {
                    let right = take(&mut stack);
                    let left = take(&mut stack);
                    let (lt, rt) = (self.lir.ty(left).clone(), self.lir.ty(right).clone());
                    let (left, right) = (self.lower_adapt(left, &lt, ty), self.lower_adapt(right, &rt, ty));
                    self.lir.push(Kind::Binary { op: *arith, left, right }, ty.clone())
                }
                Op::Not => {
                    let value = take(&mut stack);
                    self.lir.push(Kind::Not(value), ErasedType::Z)
                }
                Op::Neg(ty) => {
                    let value = take(&mut stack);
                    let held = self.lir.ty(value).clone();
                    let value = self.lower_adapt(value, &held, ty);
                    self.lir.push(Kind::Neg(value), ty.clone())
                }
                Op::ThreeWay(ty, nan_greater) => {
                    let right = take(&mut stack);
                    let left = take(&mut stack);
                    self.lower_three_way(left, right, ty, *nan_greater)
                }
                Op::AsRecorded => {
                    let value = take(&mut stack);
                    let from = self.lir.ty(value).clone();
                    let to = self.recorded_erasure(e);
                    self.lower_adapt(value, &from, &to)
                }
                Op::Cast(class) => {
                    let value = take(&mut stack);
                    self.lir.push(Kind::Cast { value, op: Cast::Checked }, ErasedType::L(class.clone()))
                }
                Op::Is(class) => {
                    let value = take(&mut stack);
                    self.lir.push(Kind::Is { value, class: class.clone() }, ErasedType::Z)
                }
                Op::Erased => {
                    let value = take(&mut stack);
                    let ty = self.lir.ty(value).clone();
                    self.lir.push(Kind::Cast { value, op: Cast::Erased }, ty)
                }
                // An array's own instruction where its type is an array class, else the
                // runtime's helper.
                Op::ArrayLoad => {
                    let index = take(&mut stack);
                    let array = take(&mut stack);
                    match Self::array_element(self.lir.ty(array)) {
                        Some(elem) => self.lir.push(Kind::ArrayLoad { array, index }, elem),
                        None => self.lower_erased_array("array_apply", &[array, index]),
                    }
                }
                Op::ArrayStore => {
                    let value = take(&mut stack);
                    let index = take(&mut stack);
                    let array = take(&mut stack);
                    let held = self.lir.ty(value).clone();
                    match Self::array_element(self.lir.ty(array)) {
                        Some(elem) => {
                            let value = self.lower_adapt(value, &held, &elem);
                            self.lir.push(Kind::ArrayStore { array, index, value }, ErasedType::V)
                        }
                        None => {
                            let value = self.lower_adapt(value, &held, &ErasedType::object());
                            self.lower_erased_array("array_update", &[array, index, value])
                        }
                    }
                }
                Op::ArrayLength => {
                    let array = take(&mut stack);
                    match Self::array_element(self.lir.ty(array)) {
                        Some(_) => self.lir.push(Kind::ArrayLength(array), ErasedType::I),
                        None => self.lower_erased_array("array_length", &[array]),
                    }
                }
                Op::ArrayClone => {
                    let array = take(&mut stack);
                    let of = self.lir.ty(array).clone();
                    match Self::array_element(&of) {
                        Some(_) => self.lir.push(Kind::ArrayClone(array), of),
                        None => self.lower_erased_array("array_clone", &[array]),
                    }
                }
                Op::NewArray => {
                    let length = take(&mut stack);
                    let array = self.recorded_erasure(e);
                    self.lir.push(Kind::NewArray { length }, array)
                }
                Op::BuilderAppend | Op::BuilderInsert => {
                    let value = take(&mut stack);
                    let mut operands = Vec::with_capacity(2);
                    let mut params = Vec::with_capacity(2);
                    if matches!(op, Op::BuilderInsert) {
                        let index = take(&mut stack);
                        operands.push(index);
                        params.push(ErasedType::I);
                    }
                    let builder = take(&mut stack);
                    let held = self.lir.ty(value).clone();
                    let param = Self::rendered_as(&held);
                    operands.push(self.lower_adapt(value, &held, &param));
                    params.push(param);
                    let name = if matches!(op, Op::BuilderInsert) { "insert" } else { "append" };
                    let method = Rc::new(reference(Invoke::Virtual, STRING_BUILDER, name, params, ErasedType::L(Rc::from(STRING_BUILDER))));
                    let args = self.lir.list(&operands);
                    self.lir.push(Kind::Call { method: method.clone(), at: None, receiver: builder, args, order: None }, method.ret.clone())
                }
                Op::Drop => {
                    let value = take(&mut stack);
                    let from = self.lir.ty(value).clone();
                    self.lower_adapt(value, &from, &ErasedType::V)
                }
                Op::Dup => {
                    self.hold_under(&mut stack, 1, &mut statements);
                    let value = take(&mut stack);
                    let ty = self.lir.ty(value).clone();
                    let held = self.lir.temp();
                    statements.push(self.lir.keep(held, value));
                    stack.push(self.lir.kept(held, ty.clone()));
                    self.lir.kept(held, ty)
                }
                Op::Statement => {
                    self.hold_under(&mut stack, 1, &mut statements);
                    let value = take(&mut stack);
                    let from = self.lir.ty(value).clone();
                    statements.push(self.lower_adapt(value, &from, &ErasedType::V));
                    continue;
                }
                // What follows the template's use is dead: the type its place wants is the
                // fitting's to make from none.
                Op::Throw => {
                    self.hold_under(&mut stack, 1, &mut statements);
                    let value = take(&mut stack);
                    stack.clear();
                    self.lir.push(Kind::Throw(value), ErasedType::V)
                }
            };
            stack.push(node);
        }
        let value = take(&mut stack);
        match statements.is_empty() {
            true => value,
            false => self.lir.seq(&statements, value),
        }
    }

    /// `lcmp`, `dcmpl`, `dcmpg` as the nodes of what they compute: the two values held in
    /// temporaries, the left first, then -1 where the left is less, 0 where they are equal (`+0.0`
    /// and `-0.0` among them) and 1 where it is greater; a NaN, in no order and equal to nothing,
    /// falls to the last answer, which is 1 where `nan_greater` (`dcmpg`) and -1 otherwise (`dcmpl`,
    /// whose chain asks for greater first).
    fn lower_three_way(&mut self, left: LirId, right: LirId, ty: &ErasedType, nan_greater: bool) -> LirId {
        let (lt, rt) = (self.lir.ty(left).clone(), self.lir.ty(right).clone());
        let (left, right) = (self.lower_adapt(left, &lt, ty), self.lower_adapt(right, &rt, ty));
        let (l, r) = (self.lir.temp(), self.lir.temp());
        let keeps = [self.lir.keep(l, left), self.lir.keep(r, right)];
        let (first, answer, last) = if nan_greater { (Relation::Lt, -1, 1) } else { (Relation::Gt, 1, -1) };
        let compare = |this: &mut Self, relation: Relation| {
            let (a, b) = (this.lir.kept(l, ty.clone()), this.lir.kept(r, ty.clone()));
            this.lir.push(Kind::Compare { relation, how: Comparing::Numbers, left: a, right: b }, ErasedType::Z)
        };
        let equal = compare(self, Relation::Eq);
        let (zero, last) = (self.lir.push(Kind::Int(0), ErasedType::I), self.lir.push(Kind::Int(last), ErasedType::I));
        let inner = self.lir.push(Kind::If { cond: equal, then: zero, otherwise: Some(last) }, ErasedType::I);
        let ordered = compare(self, first);
        let answer = self.lir.push(Kind::Int(answer), ErasedType::I);
        let value = self.lir.push(Kind::If { cond: ordered, then: answer, otherwise: Some(inner) }, ErasedType::I);
        self.lir.seq(&keeps, value)
    }

    /// The first of a tree's instructions that does not take the values the use gives it, None where
    /// all do: the arguments its text
    /// leaves untyped are at their own types (`Gen::placeholder_type`), which a conversion must
    /// convert by its instruction, an operation must be of (an `Int`'s of a smaller integral type
    /// or a boolean), an index must be an `Int` and a cast or a test a reference.
    fn tree_unfit(&mut self, tree: &Tree, args: &[TExprId]) -> Option<usize> {
        let int = |t: &ErasedType| matches!(t, ErasedType::I | ErasedType::Z | ErasedType::B | ErasedType::S | ErasedType::C);
        let mut stack: Vec<ErasedType> = Vec::new();
        for (k, op) in tree.ops.iter().enumerate() {
            let mut take = |n: usize| stack.split_off(stack.len().saturating_sub(n));
            let left = match op {
                Op::Operand(n, declared) => declared.clone().unwrap_or_else(|| self.placeholder_type(args[*n])),
                Op::Call(m) => {
                    take(m.params.len() + 1);
                    m.ret.clone()
                }
                Op::StaticCall(m) => {
                    take(m.params.len());
                    m.ret.clone()
                }
                Op::New(c) => {
                    take(c.params.len());
                    ErasedType::L(c.owner.clone())
                }
                Op::StaticField(_, _, ty) => ty.clone(),
                Op::Field(_, _, ty) => {
                    take(1);
                    ty.clone()
                }
                Op::FieldStore(..) | Op::StaticFieldStore(..) => {
                    take(if matches!(op, Op::FieldStore(..)) { 2 } else { 1 });
                    ErasedType::V
                }
                Op::Neg(ty) => {
                    if !take(1).iter().all(|t| t == ty || *ty == ErasedType::I && int(t)) {
                        return Some(k);
                    }
                    ty.clone()
                }
                Op::ThreeWay(ty, _) => {
                    if !take(2).iter().all(|t| t == ty) {
                        return Some(k);
                    }
                    ErasedType::I
                }
                Op::ArrayStore | Op::Drop | Op::Throw => {
                    take(if matches!(op, Op::ArrayStore) { 3 } else { 1 });
                    ErasedType::V
                }
                Op::Module(class) => ErasedType::L(class.clone()),
                Op::Int(_) => ErasedType::I,
                Op::Null | Op::AsRecorded => {
                    if matches!(op, Op::AsRecorded) {
                        take(1);
                    }
                    ErasedType::object()
                }
                Op::Text(_) => ErasedType::L(Rc::from(STRING)),
                Op::Convert(to, opcode) => {
                    let from = take(1).pop().unwrap_or(ErasedType::V);
                    if primitive_conversion(&from, to) != [*opcode] {
                        return Some(k);
                    }
                    to.clone()
                }
                Op::Binary(_, ty) => {
                    if !take(2).iter().all(|t| t == ty || *ty == ErasedType::I && int(t)) {
                        return Some(k);
                    }
                    ty.clone()
                }
                Op::Not => {
                    if take(1).first() != Some(&ErasedType::Z) {
                        return Some(k);
                    }
                    ErasedType::Z
                }
                Op::Cast(class) | Op::Is(class) => {
                    if !take(1).first().is_some_and(|t| t.is_ref()) {
                        return Some(k);
                    }
                    if matches!(op, Op::Is(_)) { ErasedType::Z } else { ErasedType::L(class.clone()) }
                }
                Op::Erased | Op::ArrayClone => take(1).pop().unwrap_or(ErasedType::V),
                Op::ArrayLoad => {
                    let taken = take(2);
                    if !taken.get(1).is_some_and(int) {
                        return Some(k);
                    }
                    taken.first().and_then(Self::array_element).unwrap_or_else(ErasedType::object)
                }
                Op::ArrayLength => {
                    take(1);
                    ErasedType::I
                }
                Op::NewArray => {
                    if !take(1).first().is_some_and(int) {
                        return Some(k);
                    }
                    ErasedType::object()
                }
                Op::BuilderAppend | Op::BuilderInsert => {
                    let taken = take(if matches!(op, Op::BuilderInsert) { 3 } else { 2 });
                    if matches!(op, Op::BuilderInsert) && !taken.get(1).is_some_and(int) {
                        return Some(k);
                    }
                    ErasedType::L(Rc::from(STRING_BUILDER))
                }
                Op::Dup => stack.last().cloned().unwrap_or(ErasedType::V),
                Op::Statement => {
                    take(1);
                    continue;
                }
            };
            stack.push(left);
        }
        None
    }

    /// The values under the top `top` of a template's stack, evaluated before them, each held in a
    /// temporary in their order (a constant or a temporary's read as it is): the statement or the
    /// temporary made of the top then comes after them, as their instructions do.
    fn hold_under(&mut self, stack: &mut [LirId], top: usize, statements: &mut Vec<LirId>) {
        let under = stack.len().saturating_sub(top);
        for value in &mut stack[..under] {
            let node = self.lir.node(*value);
            if matches!(node.kind, Kind::Int(_) | Kind::Long(_) | Kind::Float(_) | Kind::Double(_) | Kind::Bool(_) | Kind::Char(_) | Kind::Str(_) | Kind::Null | Kind::Temp(_)) {
                continue;
            }
            let ty = node.ty.clone();
            let held = self.lir.temp();
            statements.push(self.lir.keep(held, *value));
            *value = self.lir.kept(held, ty);
        }
    }

    /// An operation on an array whose kind only the value knows: the runtime's helper
    /// (`erased_array_method`).
    fn lower_erased_array(&mut self, name: &str, operands: &[LirId]) -> LirId {
        let method = Rc::new(erased_array_method(name));
        let ret = method.ret.clone();
        let args = self.lir.list(operands);
        self.lir.push(Kind::StaticCall { method, args, order: None }, ret)
    }
}

/// The type a placeholder declares: `L` is `Object`, anything else a descriptor.
pub(super) fn declared_type(text: &str) -> ErasedType {
    match text {
        "L" => ErasedType::object(),
        d => parse_type(d).0,
    }
}

fn read_template(text: &str) -> Template {
    let tokens = tokenize(text);
    if !single_form(text, &tokens) {
        let mut at = 0;
        return match read_tree(&tokens, &mut at) {
            Some(tree) => Template::Tree(Rc::new(tree)),
            None => Template::Unread(at),
        };
    }
    let kind = match tokens[0].as_str() {
        "invokestatic" => Invoke::Static,
        "invokevirtual" => Invoke::Virtual,
        "invokeinterface" => Invoke::Interface,
        // A class's own method called directly: no template is one.
        _ => return Template::Unread(0),
    };
    match member(kind, &tokens[1]) {
        Some(m) => Template::Single(Rc::new(m)),
        None => Template::Unread(0),
    }
}

/// The member of `Object` and its result for the typer's text of `super.toString` and the like where
/// no class behind defines the member (`Typer::apply_super_any_member`); None for any other text.
fn any_member_over_super(text: &str) -> Option<(&'static str, ErasedType)> {
    Some(match text {
        "$anyStr($0)" => ("toString", ErasedType::L(Rc::from(STRING))),
        "$identityHash($0)" => ("hashCode", ErasedType::I),
        "$cloneObject($0)" => ("clone", ErasedType::object()),
        _ => return None,
    })
}

/// What a refusal of stack code no tree is read from names, in the old assembler's words: the
/// instruction at token `at` where reading stopped, or the placeholder, the member or the field
/// reference, or the runtime call it could not read.
fn unread(tokens: &[String], at: usize) -> (&'static str, String) {
    let t = tokens.get(at).map_or("", |t| t.as_str());
    let item = tokens.get(at + 1).cloned().unwrap_or_default();
    match t {
        _ if t.starts_with('$') && t[1..].split(':').next().is_none_or(|n| n.parse::<usize>().is_err()) => ("placeholder", t.to_string()),
        "invokestatic" | "invokevirtual" | "invokeinterface" | "invokespecial" if split_method(&item).is_none() => ("method reference", item),
        "getstatic" | "getfield" | "putstatic" | "putfield" if item.split_once(':').and_then(|(o, _)| o.rsplit_once('.')).is_none() => ("field reference", item),
        "rtcall" if !item.contains('(') => ("runtime call", item),
        _ => ("instruction", t.to_string()),
    }
}

/// `owner.name(desc)ret` as a method's reference; None for a text that is none, or whose
/// signature is not spelled as a descriptor is.
fn member(kind: Invoke, text: &str) -> Option<MethodRef> {
    let (owner, name, desc) = split_method(text)?;
    method(kind, owner, name, desc)
}

/// A method by its erased signature.
fn reference(kind: Invoke, owner: &str, name: &str, params: Vec<ErasedType>, ret: ErasedType) -> MethodRef {
    let desc = method_desc(&params, &ret);
    MethodRef { owner: Rc::from(owner), owner_is_interface: kind == Invoke::Interface, name: name.to_string(), params, ret, desc, kind }
}

fn method(kind: Invoke, owner: &str, name: &str, desc: &str) -> Option<MethodRef> {
    let (params, ret) = parse_method_desc(desc);
    if method_desc(&params, &ret) != desc {
        return None;
    }
    Some(MethodRef { owner: Rc::from(owner), owner_is_interface: kind == Invoke::Interface, name: name.to_string(), params, ret, desc: desc.to_string(), kind })
}

/// What stack code has left, as far as its text says.
enum Slot {
    /// A value, with its type where the text fixes it (an argument at its static type and
    /// `cast_result` have the use's).
    Value(Option<ErasedType>),
    /// `new C dup`, until `C`'s constructor is called on it.
    Raw(Rc<str>),
}

fn value(stack: &mut Vec<Slot>) -> Option<Option<ErasedType>> {
    match stack.pop()? {
        Slot::Value(t) => Some(t),
        Slot::Raw(_) => None,
    }
}

fn values(stack: &mut Vec<Slot>, n: usize) -> Option<()> {
    for _ in 0..n {
        value(stack)?;
    }
    Some(())
}

/// After an instruction that leaves no value: with more instructions after it, a statement of the
/// template's block (its value taken off the stack); else the template's value, the last.
fn no_value(stack: &mut Vec<Slot>, ops: &mut Vec<Op>, more: bool) -> bool {
    if more {
        stack.pop();
        ops.push(Op::Statement);
    }
    !more
}

/// A value the text types as an `Int`, or one whose type is the use's argument's.
fn int(stack: &mut Vec<Slot>) -> Option<()> {
    matches!(value(stack)?, None | Some(ErasedType::I)).then_some(())
}

/// A value the text types as a reference, or one whose type is the use's argument's.
fn reference_value(stack: &mut Vec<Slot>) -> Option<()> {
    value(stack)?.is_none_or(|t| t.is_ref()).then_some(())
}

/// Stack code as a tree, its instructions run over a model of the stack: each takes its operands'
/// nodes off it and puts its own on; a `dup` holds its value in a temporary read twice, and an
/// instruction that leaves no value with more after it (a `pop`, a call, a field or an element
/// set) makes a statement of the template's block. None for code that is none (an instruction
/// that finds too little, one after the last value, more than one value left).
/// `at` is left at the token of the instruction being read, where reading stops when it does.
fn read_tree(tokens: &[String], at: &mut usize) -> Option<Tree> {
    const ARITHS: [Arith; 11] = [Arith::Add, Arith::Sub, Arith::Mul, Arith::Div, Arith::Rem, Arith::And, Arith::Or, Arith::Xor, Arith::Shl, Arith::Shr, Arith::Ushr];
    let mut ops: Vec<Op> = Vec::new();
    let mut op_at: Vec<usize> = Vec::new();
    let mut stack: Vec<Slot> = Vec::new();
    let mut placeholders: Vec<usize> = Vec::new();
    // Nothing follows an instruction that leaves no value.
    let mut ended = false;
    let mut makes_array = false;
    let mut i = 0;
    while i < tokens.len() {
        *at = i;
        // Each op read so far is of the instruction before this one.
        op_at.resize(ops.len(), *at);
        if ended {
            return None;
        }
        let t = tokens[i].as_str();
        i += 1;
        if let Some(rest) = t.strip_prefix('$') {
            let (index, declared) = match rest.split_once(':') {
                Some((n, ty)) => (n, Some(declared_type(ty))),
                None => (rest, None),
            };
            // An argument named twice is evaluated twice, as its instructions load it twice; one
            // at `V` is evaluated as a statement.
            let n: usize = index.parse().ok()?;
            placeholders.push(n);
            ops.push(Op::Operand(n, declared.clone()));
            match declared == Some(ErasedType::V) {
                true => ops.push(Op::Statement),
                false => stack.push(Slot::Value(declared)),
            }
            continue;
        }
        let mut operand = || {
            let o = tokens.get(i).map(|o| o.as_str());
            i += 1;
            o
        };
        match t {
            "invokestatic" | "invokevirtual" | "invokeinterface" | "rtcall" => {
                let member_text = operand()?;
                let m = match t {
                    "invokestatic" => member(Invoke::Static, member_text)?,
                    "invokevirtual" => member(Invoke::Virtual, member_text)?,
                    "invokeinterface" => member(Invoke::Interface, member_text)?,
                    _ => {
                        let (name, desc) = member_text.split_at(member_text.find('(')?);
                        method(Invoke::Virtual, RUNTIME_MODULE, name, desc)?
                    }
                };
                values(&mut stack, m.params.len() + (m.kind != Invoke::Static) as usize)?;
                let leaves = m.ret.clone();
                stack.push(Slot::Value(Some(m.ret.clone())));
                ops.push(if m.kind == Invoke::Static { Op::StaticCall(Rc::new(m)) } else { Op::Call(Rc::new(m)) });
                if leaves == ErasedType::V {
                    ended = no_value(&mut stack, &mut ops, i < tokens.len());
                }
            }
            "invokespecial" => {
                let ctor = member(Invoke::Special, operand()?)?;
                if ctor.name != "<init>" || ctor.ret != ErasedType::V {
                    return None;
                }
                values(&mut stack, ctor.params.len())?;
                match stack.pop()? {
                    Slot::Raw(class) if class == ctor.owner => {}
                    _ => return None,
                }
                stack.push(Slot::Value(Some(ErasedType::L(ctor.owner.clone()))));
                ops.push(Op::New(Rc::new(ctor)));
            }
            "new" => {
                let class = operand()?;
                if operand() != Some("dup") {
                    return None;
                }
                stack.push(Slot::Raw(Rc::from(class)));
            }
            "getfield" | "putfield" | "putstatic" => {
                let (owner_name, desc) = operand()?.split_once(':')?;
                let (owner, name) = owner_name.rsplit_once('.')?;
                let ty = parse_type(desc).0;
                if ty == ErasedType::V {
                    return None;
                }
                let (owner, name) = (Rc::from(owner), Rc::from(name));
                match t {
                    "getfield" => {
                        reference_value(&mut stack)?;
                        stack.push(Slot::Value(Some(ty.clone())));
                        ops.push(Op::Field(owner, name, ty));
                    }
                    _ => {
                        value(&mut stack)?;
                        if t == "putfield" {
                            reference_value(&mut stack)?;
                        }
                        stack.push(Slot::Value(Some(ErasedType::V)));
                        ops.push(if t == "putfield" { Op::FieldStore(owner, name, ty) } else { Op::StaticFieldStore(owner, name, ty) });
                        ended = no_value(&mut stack, &mut ops, i < tokens.len());
                    }
                }
            }
            "getstatic" => {
                let (owner_name, desc) = operand()?.split_once(':')?;
                let (owner, name) = owner_name.rsplit_once('.')?;
                let ty = parse_type(desc).0;
                if ty == ErasedType::V {
                    return None;
                }
                stack.push(Slot::Value(Some(ty.clone())));
                if name == "MODULE$" && ty == ErasedType::L(Rc::from(owner)) {
                    ops.push(Op::Module(Rc::from(owner)));
                } else {
                    ops.push(Op::StaticField(Rc::from(owner), Rc::from(name), ty));
                }
            }
            "rt" => {
                stack.push(Slot::Value(Some(ErasedType::L(Rc::from(RUNTIME_MODULE)))));
                ops.push(Op::Module(Rc::from(RUNTIME_MODULE)));
            }
            "checkcast" => {
                let class = operand()?;
                reference_value(&mut stack)?;
                stack.push(Slot::Value(Some(ErasedType::L(Rc::from(class)))));
                ops.push(Op::Cast(Rc::from(class)));
            }
            "instanceof" => {
                let class = operand()?;
                reference_value(&mut stack)?;
                stack.push(Slot::Value(Some(ErasedType::Z)));
                ops.push(Op::Is(Rc::from(class)));
            }
            // The array's kind is its type's at the use: what is left has none in the text.
            "array_apply" => {
                int(&mut stack)?;
                value(&mut stack)?;
                stack.push(Slot::Value(None));
                ops.push(Op::ArrayLoad);
            }
            "array_update" => {
                value(&mut stack)?;
                int(&mut stack)?;
                value(&mut stack)?;
                stack.push(Slot::Value(Some(ErasedType::V)));
                ops.push(Op::ArrayStore);
                ended = no_value(&mut stack, &mut ops, i < tokens.len());
            }
            "array_length" => {
                value(&mut stack)?;
                stack.push(Slot::Value(Some(ErasedType::I)));
                ops.push(Op::ArrayLength);
            }
            "array_clone" => {
                value(&mut stack)?;
                stack.push(Slot::Value(None));
                ops.push(Op::ArrayClone);
            }
            "array_new" => {
                int(&mut stack)?;
                makes_array = true;
                stack.push(Slot::Value(None));
                ops.push(Op::NewArray);
            }
            "builder_append" | "builder_insert" => {
                value(&mut stack)?;
                if t == "builder_insert" {
                    int(&mut stack)?;
                }
                value(&mut stack)?;
                stack.push(Slot::Value(Some(ErasedType::L(Rc::from(STRING_BUILDER)))));
                ops.push(if t == "builder_append" { Op::BuilderAppend } else { Op::BuilderInsert });
            }
            "aconst_null" => {
                stack.push(Slot::Value(Some(ErasedType::object())));
                ops.push(Op::Null);
            }
            "ldc" => {
                let text = operand()?;
                stack.push(Slot::Value(Some(ErasedType::L(Rc::from(STRING)))));
                ops.push(Op::Text(Rc::from(text)));
            }
            "ipush" => {
                let n: i32 = operand()?.parse().ok()?;
                stack.push(Slot::Value(Some(ErasedType::I)));
                ops.push(Op::Int(n));
            }
            "cast_result" => {
                value(&mut stack)?;
                // Its type is the use's: nothing whose reading depends on it follows.
                if i != tokens.len() {
                    return None;
                }
                stack.push(Slot::Value(None));
                ops.push(Op::AsRecorded);
            }
            // A `dup`: the template's block holds the value in a temporary and reads it twice (the
            // class-file writer keeps it on the stack where it can, `Gen::kept_on_stack`).
            "dup" => {
                let top = value(&mut stack)?;
                stack.push(Slot::Value(top.clone()));
                stack.push(Slot::Value(top));
                ops.push(Op::Dup);
            }
            "pop" if stack.len() > 1 || i < tokens.len() => {
                value(&mut stack)?;
                ops.push(Op::Statement);
            }
            "pop" => {
                value(&mut stack)?;
                ended = true;
                stack.push(Slot::Value(Some(ErasedType::V)));
                ops.push(Op::Drop);
            }
            // What follows a throw never runs: it is not read, and the throw is the template's
            // value, the values under it evaluated before it.
            "athrow" => {
                value(&mut stack)?;
                stack.clear();
                stack.push(Slot::Value(Some(ErasedType::V)));
                ops.push(Op::Throw);
                op_at.resize(ops.len(), *at);
                break;
            }
            _ => match plain_op(t)? {
                (opcode, 0, Some(ErasedType::I)) => {
                    // An iconst instruction: their opcodes stand between aconst_null's and lconst_0's.
                    let v = (op::ACONST_NULL < opcode && opcode < op::LCONST_0).then(|| opcode as i32 - op::ICONST_0 as i32)?;
                    stack.push(Slot::Value(Some(ErasedType::I)));
                    ops.push(Op::Int(v));
                }
                // A conversion, where the adaptation between the two types is that instruction.
                (opcode, 1, Some(ty)) if matches!(opcode, op::INEG | op::LNEG | op::DNEG) => {
                    if value(&mut stack)?.is_some_and(|t| t != ty) {
                        return None;
                    }
                    stack.push(Slot::Value(Some(ty.clone())));
                    ops.push(Op::Neg(ty));
                }
                // Check a known operand's conversion here. `tree_unfit` checks an operand typed at the use.
                (opcode, 1, Some(to)) => {
                    if let Some(from) = value(&mut stack)? {
                        if primitive_conversion(&from, &to) != [opcode] {
                            return None;
                        }
                    }
                    stack.push(Slot::Value(Some(to.clone())));
                    ops.push(Op::Convert(to, opcode));
                }
                (opcode, 2, Some(ErasedType::I)) if matches!(opcode, op::LCMP | op::DCMPL | op::DCMPG) => {
                    let ty = if opcode == op::LCMP { ErasedType::J } else { ErasedType::D };
                    let (right, left) = (value(&mut stack)?, value(&mut stack)?);
                    if left.is_some_and(|t| t != ty) || right.is_some_and(|t| t != ty) {
                        return None;
                    }
                    stack.push(Slot::Value(Some(ErasedType::I)));
                    ops.push(Op::ThreeWay(ty, opcode == op::DCMPG));
                }
                (opcode, 2, Some(ty)) => {
                    let (right, left) = (value(&mut stack)?, value(&mut stack)?);
                    if opcode == op::IXOR && left == Some(ErasedType::Z) && matches!(ops.last(), Some(Op::Int(1))) {
                        ops.pop();
                        stack.push(Slot::Value(Some(ErasedType::Z)));
                        ops.push(Op::Not);
                        continue;
                    }
                    let arith = ARITHS.into_iter().find(|a| arith_instruction(*a, &ty) == opcode)?;
                    if left.is_some_and(|t| t != ty) || right.is_some_and(|t| t != ty) {
                        return None;
                    }
                    stack.push(Slot::Value(Some(ty.clone())));
                    ops.push(Op::Binary(arith, ty));
                }
                _ => return None,
            },
        }
    }
    op_at.resize(ops.len(), *at);
    if stack.len() != 1 || !matches!(stack[0], Slot::Value(_)) {
        return None;
    }
    if ops.is_empty() {
        return None;
    }
    // An argument alone is the argument at its own type or at a primitive type the template
    // declares, or at a reference type it declares, a cast erasure drops.
    if matches!(ops.as_slice(), [Op::Operand(_, Some(declared))] if declared.is_ref()) {
        ops.push(Op::Erased);
        op_at.push(*at);
    }
    let arguments = placeholders.iter().try_fold(0, |most: usize, &n| Some(most.max(n.checked_add(1)?)))?;
    Some(Tree { ops, arguments, at: op_at, makes_array })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The node kinds a template's tree is made of, in its instructions' order.
    fn tree_kinds(tree: &Tree) -> String {
        let kinds: Vec<&str> = tree
            .ops
            .iter()
            .filter_map(|op| match op {
                Op::Operand(..) => None,
                Op::Call(_) => Some("Call"),
                Op::StaticCall(_) => Some("StaticCall"),
                Op::New(_) => Some("New"),
                Op::StaticField(..) => Some("StaticField"),
                Op::Field(..) => Some("Field"),
                Op::FieldStore(..) => Some("FieldStore"),
                Op::StaticFieldStore(..) => Some("StaticFieldStore"),
                Op::Text(_) => Some("Text"),
                Op::Module(_) => Some("Module"),
                Op::Int(_) => Some("Int"),
                Op::Null => Some("Null"),
                Op::Convert(..) | Op::AsRecorded => Some("Adapt"),
                Op::Binary(..) => Some("Binary"),
                Op::Not => Some("Not"),
                Op::Neg(_) => Some("Neg"),
                Op::ThreeWay(..) => Some("Block(If)"),
                Op::Cast(_) => Some("Cast"),
                Op::Is(_) => Some("Is"),
                Op::Erased => Some("Cast(erased)"),
                Op::ArrayLoad => Some("ArrayLoad"),
                Op::ArrayStore => Some("ArrayStore"),
                Op::ArrayLength => Some("ArrayLength"),
                Op::ArrayClone => Some("ArrayClone"),
                Op::NewArray => Some("NewArray"),
                Op::BuilderAppend | Op::BuilderInsert => Some("Call"),
                Op::Drop => Some("Drop"),
                Op::Dup => Some("Block(temporary)"),
                Op::Statement => Some("Block(statement)"),
                Op::Throw => Some("Throw"),
            })
            .collect();
        kinds.join(" ")
    }

    /// Every `@jvm("...")` text of the std's sources with the place of its first use and how many
    /// declarations carry it, the literal's escapes read as the typer reads them; and each source read
    /// with the number of declarations it holds.
    fn std_templates() -> (Vec<(String, String, usize)>, Vec<(String, usize)>) {
        fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
            let mut entries: Vec<_> = std::fs::read_dir(dir).expect("the std's sources").map(|e| e.expect("an entry").path()).collect();
            entries.sort();
            for path in entries {
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|x| x == "scala") {
                    out.push(path);
                }
            }
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        walk(&root.join("std"), &mut files);
        let mut found: Vec<(String, String, usize)> = Vec::new();
        let mut read = Vec::new();
        for file in files {
            let source = std::fs::read_to_string(&file).expect("a std source");
            let name = file.strip_prefix(root).expect("under the root").display().to_string();
            read.push((name.clone(), source.matches("@jvm(\"").count()));
            for (n, line) in source.lines().enumerate() {
                let mut rest = line;
                while let Some(at) = rest.find("@jvm(\"") {
                    let mut text = String::new();
                    let mut chars = rest[at + 6..].chars();
                    while let Some(c) = chars.next() {
                        match c {
                            '"' => break,
                            '\\' => match chars.next() {
                                Some('n') => text.push('\n'),
                                Some('t') => text.push('\t'),
                                Some(other) => text.push(other),
                                None => {}
                            },
                            _ => text.push(c),
                        }
                    }
                    rest = chars.as_str();
                    match found.iter_mut().find(|(t, _, _)| *t == text) {
                        Some(row) => row.2 += 1,
                        None => found.push((text, format!("{}:{}", name, n + 1), 1)),
                    }
                }
            }
        }
        (found, read)
    }

    /// Every code template of the std is a node or nodes of the lowered IR: one member's call, a tree
    /// of nodes, or a typer's text with a route; a text no node is made of would be refused at each
    /// use and is listed here with the reason, so that a new one fails. A use is still refused where
    /// it gives too few arguments, arguments the instructions do not take, or makes an array of a
    /// type that is none (`Gen::template_route`). With
    /// `TEQ_TEMPLATE_TABLE=<file>`, the table of every template and what it lowers to is written there.
    #[test]
    fn std_templates_are_nodes() {
        // The texts no node is made of yet: none.
        const LEFT: [&str; 0] = [];
        let (templates, read) = std_templates();
        // The walk read the std whole: each source the compiler embeds (its layers' tables) is among the files
        // read with the declarations it holds, and every declaration read is a use of a template.
        for (path, text) in crate::STD_SOURCES.iter().chain(crate::JVM_STD_SOURCES).chain(crate::SCALA_LIBRARY_LAYER).chain(crate::SCALAJS_SOURCES) {
            let name = path.replacen("<std>", "std", 1);
            let walked = read.iter().find(|(n, _)| *n == name).map(|&(_, k)| k);
            assert_eq!(walked, Some(text.matches("@jvm(\"").count()), "{name}: the declarations read against the embedded source's");
        }
        assert_eq!(templates.iter().map(|t| t.2).sum::<usize>(), read.iter().map(|r| r.1).sum::<usize>(), "every declaration read as a template's use");
        let mut rows = Vec::new();
        let mut without_node: Vec<&str> = Vec::new();
        for (text, at, uses) in &templates {
            let lowered = if text.starts_with('!') || typer_text(text) {
                "a typer's text".to_string()
            } else {
                match read_template(text) {
                    Template::Single(m) => (if m.kind == Invoke::Static { "StaticCall" } else { "Call" }).to_string(),
                    Template::Tree(tree) => format!("tree: {}", tree_kinds(&tree)),
                    Template::Unread(at) => {
                        without_node.push(text);
                        let (what, item) = unread(&tokenize(text), at);
                        format!("refused: the {} {}", what, item)
                    }
                }
            };
            rows.push(format!("{}\t{}\t{}\t{}", text.replace('\n', "\\n"), at, uses, lowered));
        }
        if let Some(file) = std::env::var_os("TEQ_TEMPLATE_TABLE") {
            std::fs::write(file, rows.join("\n") + "\n").expect("the table written");
        }
        // Exactly the listed texts: a new one fails, and so does one listed that is a node now or gone.
        without_node.sort_unstable();
        let mut listed = LEFT.to_vec();
        listed.sort_unstable();
        assert_eq!(without_node, listed, "the std's templates no node is made of, against the list");
    }
}
