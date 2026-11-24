//! The `@jvm` templates as nodes of the lowered IR (`crate::lir`). A template's text is read
//! once; a use whose text is a tree becomes nodes over its operands, and every other use is
//! assembled from the text (`Gen::assemble`). Of the texts the typer makes itself, one that
//! is a call becomes that call; `Gen::intrinsic` writes the others.

use crate::lir::{Arith, Cast, ErasedType, Invoke, Kind, LirId, MethodRef, Unlowered};
use super::classfile::op;
use super::gen::{erased_array_method, primitive_conversion, Gen, Referenced, Site};
use super::intrinsic::{plain_op, single_form, split_method, tokenize, typer_text};
use super::names::*;
use super::write::arith_instruction;
use crate::ast::ListRef;
use crate::symbols::Owner;
use crate::tir::*;
use crate::types::{ClassId, Type};
use std::rc::Rc;

/// What a template's text is, whatever uses it.
pub(super) enum Template {
    /// One member reference and nothing else (`single_form`): its operands are the last of the
    /// arguments, as many as the reference takes.
    Single(Rc<MethodRef>),
    /// Stack code that is a tree.
    Tree(Rc<Tree>),
    /// Other stack code, or a text the typer made.
    Text,
}

/// Stack code in which every value is left once and taken once, by the instruction that
/// follows its operands: the instructions in the text's order are a tree's nodes, operands
/// first.
pub(super) struct Tree {
    ops: Vec<Op>,
    /// How many arguments the placeholders reach.
    arguments: usize,
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
    /// `rt`, `getstatic C.MODULE$:LC;`: the instance of the object whose class is `C`.
    Module(Rc<str>),
    /// `ipush n`, `iconst_n`.
    Int(i32),
    /// `aconst_null`: the null reference.
    Null,
    /// A primitive conversion, to this type.
    Convert(ErasedType),
    /// An operation on two values of this type.
    Binary(Arith, ErasedType),
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
    /// From the text: assembled, or the typer's own text.
    Text,
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
            return self.typer_route(template, text, args).unwrap_or(TemplateRoute::Text);
        }
        match &*self.template(text) {
            Template::Single(m) if args.len() >= m.params.len() + (m.kind != Invoke::Static) as usize => TemplateRoute::Single(m.clone()),
            Template::Tree(tree) if args.len() >= tree.arguments && (!tree.makes_array || Self::array_element(&self.recorded_erasure(e)).is_some()) => {
                TemplateRoute::Tree(tree.clone())
            }
            _ => TemplateRoute::Text,
        }
    }

    /// A use of a template, or why its route is not lowered yet.
    pub(super) fn lower_template(&mut self, e: TExprId, template: StrRef, args: ListRef) -> Result<LirId, Unlowered> {
        let cx = self.cx;
        let args: &[TExprId] = cx.input.prog.expr_list(args);
        match self.template_route(e, template, args) {
            TemplateRoute::Single(m) => Ok(self.lower_single(args, &m)),
            TemplateRoute::Tree(tree) => Ok(self.lower_tree(e, args, &tree)),
            TemplateRoute::ObjectMember(m) => Ok(self.lower_object_member(args, &m)),
            TemplateRoute::Member(m) => Ok(self.lower_member(args, &m)),
            TemplateRoute::Made(ctor) => {
                let mut operands: Vec<LirId> = Vec::with_capacity(ctor.params.len());
                for (a, p) in args.iter().zip(&ctor.params) {
                    operands.push(self.lower_expr(*a, p));
                }
                let operands = self.lir.list(&operands);
                Ok(self.lir.push(Kind::New { touch: None, ctor: ctor.clone(), args: operands }, ErasedType::L(ctor.owner.clone())))
            }
            TemplateRoute::Synchronized => Ok(self.lower_synchronized(args[0], args[1])),
            TemplateRoute::Text => Err(Unlowered::TemplateText),
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
            "$getClass($0)" if !args.is_empty() && self.primitive_receiver(args[0]).is_none() => {
                TemplateRoute::ObjectMember(named(Invoke::Virtual, OBJECT, "getClass", Vec::new(), ErasedType::L(Rc::from("java/lang/Class"))))
            }
            "$synchronized($0, $1)" if args.len() >= 2 => TemplateRoute::Synchronized,
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

    /// The name of the class that declares the def a template stands on.
    pub(super) fn template_owner(&mut self, template: StrRef) -> Option<Rc<str>> {
        let cx = self.cx;
        match cx.input.syms.sym(*cx.input.prog.template_syms.get(&template)?).owner {
            Owner::Class(c) => Some(self.class_name(c)),
            _ => None,
        }
    }

    /// The primitive a receiver of `getClass` is where its static type is one: its class is
    /// the primitive's, as scalac rewrites it (`1.getClass` prints `int`, `().getClass`
    /// `void`). A value class's is its box's, which the receiver is boxed into.
    pub(super) fn primitive_receiver(&mut self, r: TExprId) -> Option<ErasedType> {
        let t = self.recorded_type(r).filter(|_| self.expr_value_class(r).is_none())?;
        let erased = self.erase(t);
        (!matches!(erased, ErasedType::L(_))).then_some(erased)
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
                Op::Module(class) => self.lir.push(Kind::Module(class.clone()), ErasedType::L(class.clone())),
                Op::Int(v) => self.lir.push(Kind::Int(*v), ErasedType::I),
                // The null seen at `Object`, as `Gen::assemble` types it: compared with `eq`/`ne` it is
                // loaded beside the other side (`aconst_null; if_acmpXX`), where a `Null` node alone is
                // tested by `ifnull`/`ifnonnull` (`Gen::write_compare`).
                Op::Null => {
                    let null = self.lir.push(Kind::Null, ErasedType::object());
                    self.lir.push(Kind::Typed(null), ErasedType::object())
                }
                Op::Convert(to) => {
                    let value = take(&mut stack);
                    let from = self.lir.ty(value).clone();
                    self.lower_adapt(value, &from, to)
                }
                Op::Binary(arith, ty) => {
                    let right = take(&mut stack);
                    let left = take(&mut stack);
                    self.lir.push(Kind::Binary { op: *arith, left, right }, ty.clone())
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
                Op::Drop => {
                    let value = take(&mut stack);
                    let from = self.lir.ty(value).clone();
                    self.lower_adapt(value, &from, &ErasedType::V)
                }
                Op::Dup => {
                    let value = take(&mut stack);
                    let ty = self.lir.ty(value).clone();
                    let held = self.lir.temp();
                    statements.push(self.lir.keep(held, value));
                    stack.push(self.lir.kept(held, ty.clone()));
                    self.lir.kept(held, ty)
                }
                Op::Statement => {
                    let value = take(&mut stack);
                    let from = self.lir.ty(value).clone();
                    statements.push(self.lower_adapt(value, &from, &ErasedType::V));
                    continue;
                }
                // What follows the template's use is dead: the type its place wants is the
                // fitting's to make from none.
                Op::Throw => {
                    let value = take(&mut stack);
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

    /// An operation on an array whose kind only the value knows: the runtime's helper
    /// (`erased_array_method`).
    fn lower_erased_array(&mut self, name: &str, operands: &[LirId]) -> LirId {
        let method = Rc::new(erased_array_method(name));
        let ret = method.ret.clone();
        let args = self.lir.list(operands);
        self.lir.push(Kind::StaticCall { method, args, order: None }, ret)
    }
}

/// Whether each read of a `dup`'s two copies is the receiver of a call, the first value the call
/// evaluates, every statement after the `dup` and the template's value being such a call (or a
/// value dropped of one): the reads the class-file writer keeps on the operand stack
/// (`Gen::kept_on_stack`). Anything else would hold the copy in a slot, which the assembler's
/// `dup` does not.
fn copies_read_first(ops: &[Op]) -> bool {
    #[derive(Clone, Copy, PartialEq)]
    enum Read {
        Other,
        Copy,
        /// A call on a copy, or such a call's value dropped.
        OnCopy,
    }
    let mut stack: Vec<Read> = Vec::new();
    let mut statements_on_copy = true;
    for op in ops {
        let takes = match op {
            Op::Operand(..) | Op::StaticField(..) | Op::Module(_) | Op::Int(_) => 0,
            Op::Call(m) => m.params.len() + 1,
            Op::StaticCall(m) | Op::New(m) => m.params.len(),
            Op::Binary(..) | Op::ArrayLoad => 2,
            Op::ArrayStore => 3,
            _ => 1,
        };
        if stack.len() < takes {
            return false;
        }
        let taken = stack.split_off(stack.len() - takes);
        // A copy is read only as a call's receiver, the first of its operands, and a value that
        // read one is no other instruction's operand: each statement and the value read one copy,
        // first.
        let receiver_copy = matches!(op, Op::Call(_)) && taken.first() == Some(&Read::Copy);
        let others = if receiver_copy { &taken[1..] } else { &taken[..] };
        let passes = matches!(op, Op::Drop | Op::AsRecorded | Op::Statement);
        if others.contains(&Read::Copy) || (!passes && others.contains(&Read::OnCopy)) {
            return false;
        }
        match op {
            Op::Dup => stack.extend([Read::Copy, Read::Copy]),
            Op::Statement => statements_on_copy &= taken[0] == Read::OnCopy,
            Op::Drop | Op::AsRecorded => stack.push(taken[0]),
            _ if receiver_copy => stack.push(Read::OnCopy),
            _ => stack.push(Read::Other),
        }
    }
    statements_on_copy && stack == [Read::OnCopy]
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
        return match read_tree(&tokens) {
            Some(tree) => Template::Tree(Rc::new(tree)),
            None => Template::Text,
        };
    }
    let kind = match tokens[0].as_str() {
        "invokestatic" => Invoke::Static,
        "invokevirtual" => Invoke::Virtual,
        "invokeinterface" => Invoke::Interface,
        // A class's own method called directly: no template is one.
        _ => return Template::Text,
    };
    match member(kind, &tokens[1]) {
        Some(m) => Template::Single(Rc::new(m)),
        None => Template::Text,
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

fn int(stack: &mut Vec<Slot>) -> Option<()> {
    (value(stack)?? == ErasedType::I).then_some(())
}

/// Stack code as a tree; None for code that is none (a value used twice, one dropped in the
/// middle, an instruction that finds too little), for an instruction no node is made of yet (the
/// builder instructions, a field of an instance or a field written, a
/// comparison), for a string constant (`ldc`, below), and for an argument alone but at a reference
/// type the template declares, whose node would be the argument's own.
fn read_tree(tokens: &[String]) -> Option<Tree> {
    const ARITHS: [Arith; 11] = [Arith::Add, Arith::Sub, Arith::Mul, Arith::Div, Arith::Rem, Arith::And, Arith::Or, Arith::Xor, Arith::Shl, Arith::Shr, Arith::Ushr];
    let mut ops: Vec<Op> = Vec::new();
    let mut stack: Vec<Slot> = Vec::new();
    let mut placeholders: Vec<usize> = Vec::new();
    // Nothing follows an instruction that leaves no value.
    let mut ended = false;
    let mut makes_array = false;
    let mut duplicated = false;
    let mut i = 0;
    while i < tokens.len() {
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
            let n: usize = index.parse().ok()?;
            // An argument named twice would be lowered twice.
            if placeholders.contains(&n) || declared == Some(ErasedType::V) {
                return None;
            }
            placeholders.push(n);
            stack.push(Slot::Value(declared.clone()));
            ops.push(Op::Operand(n, declared));
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
                ended = m.ret == ErasedType::V;
                stack.push(Slot::Value(Some(m.ret.clone())));
                ops.push(if m.kind == Invoke::Static { Op::StaticCall(Rc::new(m)) } else { Op::Call(Rc::new(m)) });
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
                if !value(&mut stack)??.is_ref() {
                    return None;
                }
                stack.push(Slot::Value(Some(ErasedType::L(Rc::from(class)))));
                ops.push(Op::Cast(Rc::from(class)));
            }
            "instanceof" => {
                let class = operand()?;
                if !value(&mut stack)??.is_ref() {
                    return None;
                }
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
                ended = true;
                stack.push(Slot::Value(Some(ErasedType::V)));
                ops.push(Op::ArrayStore);
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
            // `Gen::assemble` models the null as an `Object`, a `Null` node as the null type: the
            // two write the same bytes where no frame holds the value (the std's `nullRef`, read by
            // a branch's `areturn`), and a frame that holds it says `null` where it said `Object`.
            // Lowered as `Typed(Null)` (`lower_tree`), a comparison keeps the class file's form.
            "aconst_null" => {
                stack.push(Slot::Value(Some(ErasedType::object())));
                ops.push(Op::Null);
            }
            // A `Text` node in a concatenation is text of its recipe, where `Gen::assemble`'s
            // string is an operand of it.
            "ldc" => return None,
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
            // A `dup` of a value an instruction then takes: the template's block holds it in a
            // temporary, which the class-file writer keeps on the stack (`Gen::kept_on_stack`).
            "dup" if !duplicated => {
                let top = value(&mut stack)?;
                duplicated = true;
                stack.push(Slot::Value(top.clone()));
                stack.push(Slot::Value(top));
                ops.push(Op::Dup);
            }
            "pop" if duplicated && stack.len() > 1 => {
                value(&mut stack)?;
                ops.push(Op::Statement);
            }
            "pop" | "athrow" => {
                value(&mut stack)?;
                ended = true;
                stack.push(Slot::Value(Some(ErasedType::V)));
                ops.push(if t == "pop" { Op::Drop } else { Op::Throw });
            }
            _ => match plain_op(t)? {
                (opcode, 0, Some(ErasedType::I)) => {
                    // An iconst instruction: their opcodes stand between aconst_null's and lconst_0's.
                    let v = (op::ACONST_NULL < opcode && opcode < op::LCONST_0).then(|| opcode as i32 - op::ICONST_0 as i32)?;
                    stack.push(Slot::Value(Some(ErasedType::I)));
                    ops.push(Op::Int(v));
                }
                // A conversion, where the adaptation between the two types is that instruction.
                (opcode, 1, Some(to)) => {
                    let from = value(&mut stack)??;
                    if primitive_conversion(&from, &to) != [opcode] {
                        return None;
                    }
                    stack.push(Slot::Value(Some(to.clone())));
                    ops.push(Op::Convert(to));
                }
                (opcode, 2, Some(ty)) => {
                    let (right, left) = (value(&mut stack)??, value(&mut stack)??);
                    let arith = ARITHS.into_iter().find(|a| arith_instruction(*a, &ty) == opcode)?;
                    if left != ty || right != ty {
                        return None;
                    }
                    stack.push(Slot::Value(Some(ty.clone())));
                    ops.push(Op::Binary(arith, ty));
                }
                _ => return None,
            },
        }
    }
    if stack.len() != 1 || !matches!(stack[0], Slot::Value(_)) {
        return None;
    }
    if duplicated && !copies_read_first(&ops) {
        return None;
    }
    let root = match ops.as_slice() {
        [.., root, Op::AsRecorded] => root,
        [.., root] => root,
        [] => return None,
    };
    if matches!(root, Op::Operand(..)) {
        let [Op::Operand(_, Some(declared))] = ops.as_slice() else { return None };
        if !declared.is_ref() {
            return None;
        }
        ops.push(Op::Erased);
    }
    let arguments = placeholders.iter().try_fold(0, |most: usize, &n| Some(most.max(n.checked_add(1)?)))?;
    Some(Tree { ops, arguments, makes_array })
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
                Op::Module(_) => Some("Module"),
                Op::Int(_) => Some("Int"),
                Op::Null => Some("Null"),
                Op::Convert(_) | Op::AsRecorded => Some("Adapt"),
                Op::Binary(..) => Some("Binary"),
                Op::Cast(_) => Some("Cast"),
                Op::Is(_) => Some("Is"),
                Op::Erased => Some("Cast(erased)"),
                Op::ArrayLoad => Some("ArrayLoad"),
                Op::ArrayStore => Some("ArrayStore"),
                Op::ArrayLength => Some("ArrayLength"),
                Op::ArrayClone => Some("ArrayClone"),
                Op::NewArray => Some("NewArray"),
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
    /// of nodes, or a typer's text with a route; a text no node is made of stays the class file's own
    /// (`Kind::Legacy` of `TExpr::Js`) and is listed here with the reason, so that a new one fails.
    /// A use can still be the class file's where the template is one of these and the use gives too
    /// few arguments, or makes an array of a type that is none (`Gen::template_route`). With
    /// `TEQ_TEMPLATE_TABLE=<file>`, the table of every template and what it lowers to is written there.
    #[test]
    fn std_templates_are_nodes() {
        // The texts no node is made of yet, each with what keeps it: `newString()`'s empty string (a
        // `Str` node in a concatenation is text of its recipe, where the template's string is an
        // operand of it); `className`'s two calls with no placeholder (operands are implicit in the
        // one-member form alone); `listStorage`'s lone argument with no declared type (its node
        // would be the argument's own); the builder instructions (`builder_append`,
        // `builder_insert`: the overload follows the operand's type); `nonEmpty`'s `iconst_1 ixor`
        // on a boolean (no `Binary` of a boolean); `last`'s `dup` whose copy a call reads after
        // another call (`copies_read_first`).
        const LEFT: [&str; 7] = [
            "ldc \"\"",
            "invokevirtual java/lang/Object.getClass()Ljava/lang/Class; invokevirtual java/lang/Class.getName()Ljava/lang/String;",
            "$0",
            "$0 $1 builder_append",
            "$0 $1:I $2 builder_insert",
            "$0 invokevirtual java/lang/StringBuilder.isEmpty()Z iconst_1 ixor",
            "$0 dup invokevirtual java/lang/StringBuilder.length()I iconst_1 isub invokevirtual java/lang/StringBuilder.charAt(I)C",
        ];
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
                    Template::Text => {
                        without_node.push(text);
                        "no node: Legacy Js".to_string()
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
