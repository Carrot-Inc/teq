//! Code generation for expressions. An expression is emitted at the type its node leaves on the
//! stack, brought to the static type the typer recorded for it (a cast or an unboxing where a
//! generic result is used at a concrete type), and then to the type its consumer wants.

use super::classfile::*;
use super::free::{Analysis, Free};
use super::names::*;
use super::{Cx, Unit};
use crate::ast::{mods, ListRef};
use crate::classfile::ACC_STATIC;
use crate::intern::{FxMap, Name};
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::typer::loader::javaclass::JMember;
use crate::types::*;
use std::rc::Rc;

/// scalac's `MaxIndySlots`: a concatenation's call is written before the operand that would
/// bring it to this many operand slots.
const CONCAT_SLOTS: usize = 200;
/// What a constant of the class file holds, in bytes of modified UTF-8.
const RECIPE_BYTES: usize = 65535;

/// One call of `makeConcatWithConstants`: `\u{1}` in the recipe stands for an argument,
/// `\u{2}` for a constant that holds one of the two itself.
#[derive(Default)]
struct ConcatCall {
    recipe: String,
    bytes: usize,
    arguments: Vec<JType>,
    constants: Vec<String>,
    slots: usize,
}

impl ConcatCall {
    fn text(&mut self, text: &str) {
        self.recipe.push_str(text);
        self.bytes += recipe_bytes(text);
    }

    fn constant(&mut self, text: String) {
        self.text("\u{2}");
        self.constants.push(text);
        self.slots += 1;
    }

    fn argument(&mut self, t: JType, slots: usize) {
        self.text("\u{1}");
        self.arguments.push(t);
        self.slots += slots;
    }
}

fn recipe_bytes(text: &str) -> usize {
    text.chars().map(|c| if c == '\0' { 2 } else if c as u32 >= 0x10000 { 6 } else { c.len_utf8() }).sum()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Invoke {
    Virtual,
    Interface,
    Static,
    Special,
}

/// How a method of the program is called.
#[derive(Clone)]
pub struct MRef {
    pub owner: Rc<str>,
    pub owner_is_interface: bool,
    pub name: String,
    pub params: Vec<JType>,
    pub ret: JType,
    pub desc: String,
    pub kind: Invoke,
}

#[derive(Clone)]
pub struct Local {
    pub slot: u16,
    /// The declared type; a cell holds a value of this type as an `Object`.
    pub ty: JType,
    pub cell: bool,
}

/// How an anonymous class holds a local of the scope it stands in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Held {
    Value,
    /// A `var`, shared with its scope through its cell.
    Cell,
    /// A lazy val, as the function that reads it.
    Thunk,
}

#[derive(Clone)]
pub struct Capture {
    pub sym: SymId,
    pub cell: bool,
    pub ty: JType,
}

/// A static method made from a lambda, a local def or the initialiser of a local lazy val: the
/// captures come first, `$this` leading them when the body needs it.
pub struct Lifted {
    pub name: String,
    pub desc: String,
    pub this: bool,
    pub captures: Vec<Capture>,
    pub params: Vec<JType>,
    pub ret: JType,
}

pub enum PendingBody {
    /// The parameters arrive as `Object`s and are unboxed into locals of their declared types.
    Lambda(Vec<SymId>, TExprId),
    Fun(FunId),
    /// The default of parameter `n` of a local def.
    Default(FunId, usize),
    Lazy(SymId, TExprId),
    /// The thunk a by-name parameter's default from a jar is passed as: scalac's getter returns
    /// the value, and the argument calls it (`() => f$default$n(...)`), with the receiver and
    /// the arguments before it captured.
    JarGetter(JarGetter),
}

/// The getter of a default as scalac writes it (`Gen::default_getter_shape`).
pub struct GetterShape {
    /// The number in its name, `m$default$N`.
    pub index: usize,
    /// How many of the method's parameters, in the class file's order, it takes.
    pub before: usize,
    pub ret: JType,
    pub by_name: bool,
}

pub struct JarGetter {
    pub kind: Invoke,
    pub owner: Rc<str>,
    pub owner_is_interface: bool,
    pub name: String,
    pub desc: String,
    pub ret: JType,
}

pub struct Pending {
    pub lifted: Rc<Lifted>,
    pub body: PendingBody,
}

pub struct TailLoop {
    pub sym: SymId,
    pub params: Vec<SymId>,
    pub head: Label,
}

pub struct MethodCx {
    pub locals: FxMap<SymId, Local>,
    pub this_slot: Option<u16>,
    pub ret: JType,
    pub tail: Option<TailLoop>,
    /// The locals of the method that live in a cell (`free::cells`).
    pub cells: Vec<SymId>,
    /// Set while the arguments of the super constructor are evaluated: `this` is not there
    /// yet, and a field of it named there is the constructor parameter it will be set from.
    pub before_super: bool,
    /// The method is a lifted lambda, so a `return` in it would leave the def around it.
    pub lambda: bool,
    /// In a value class's `m$extension`, `this` is the underlying value of this type.
    pub this_underlying: Option<JType>,
    /// The `try` blocks with a finalizer the code being generated stands in, innermost last: a
    /// `return` leaves through each one's finalizer (`Gen::try_`).
    pub finalizers: Vec<FinallyFrame>,
    /// The locals of the matches being emitted that hold a value class's primitive underlying
    /// value, with the class: a type pattern tests, and a `MatchError` holds, its box.
    pub vc_slots: Vec<(u16, ClassId)>,
}

/// A `try` with a finalizer around the code being generated: `pending` is the slot a `return`
/// sets before it jumps to `exit`, where the finalizer runs once and the method returns the
/// value kept in `value` (none for a `Unit` method) or hands it on to the enclosing frame.
#[derive(Clone, Copy)]
pub struct FinallyFrame {
    pub pending: u16,
    pub value: Option<u16>,
    pub exit: Label,
}

impl Default for MethodCx {
    fn default() -> Self {
        MethodCx {
            locals: FxMap::default(),
            this_slot: None,
            ret: JType::V,
            tail: None,
            cells: Vec::new(),
            before_super: false,
            lambda: false,
            this_underlying: None,
            finalizers: Vec::new(),
            vc_slots: Vec::new(),
        }
    }
}

pub struct Gen<'a> {
    pub cx: &'a Cx<'a>,
    pub errors: Vec<String>,
    erased: Vec<Option<JType>>,
    /// Whether the erasures are kept in `erased_overlay` alone: for a Gen that emits a few units
    /// (a session's retype), whose types are the store's last ids, the table by id would be made
    /// for every type before them.
    pub sparse: bool,
    /// The erasures of the types with an overlay's id (`arena::LOCAL_BASE` on), which the parallel
    /// typer's overlays keep past the merge: a table by the id would
    /// take gigabytes per class. A sparse Gen keeps every erasure here.
    erased_overlay: FxMap<TypeId, JType>,
    class_rc: Vec<Option<Rc<str>>>,
    mrefs: FxMap<SymId, Rc<MRef>>,
    /// The function arities past 1 an erasure or a method's reference used when it was made,
    /// added again when the memo answers it, so that a unit's `function_arities` do not depend on
    /// the units the Gen emitted before it (a session keeps them per unit, `kept.rs`).
    erased_arities: FxMap<TypeId, u64>,
    mref_arities: FxMap<SymId, u64>,
    /// The fields of the traits read from class files, by trait (`class_file_trait_fields`).
    pub(super) trait_fields: FxMap<ClassId, Rc<super::classes::ClassFileTrait>>,
    pub cw: ClassWriter,
    pub this_name: Rc<str>,
    pub this_class: Option<ClassId>,
    pub is_interface: bool,
    pub abstract_class: bool,
    /// The source file of the class being written, whose lines the line table counts.
    pub line_file: FileId,
    /// The call whose arguments are being written, with its last offset (`mark_line`).
    pub line_floor: Option<(TExprId, FileId, Span, u32)>,
    /// Inside an argument the typer made, whose line holds through it (`mark_line`).
    pub line_hold: bool,
    pub pending: Vec<Pending>,
    pub lifted: FxMap<SymId, Rc<Lifted>>,
    pub lifted_defaults: FxMap<(SymId, usize), Rc<Lifted>>,
    pub an: Analysis,
    pub counter: u32,
    /// The locals an anonymous class holds in fields: symbol, field name, type, and how.
    pub class_captures: Vec<(SymId, String, JType, Held)>,
    pub code: Code,
    pub m: MethodCx,
    pub context: String,
    /// The arities of the function interfaces the emitted code names, as a bit set.
    pub function_arities: u64,
    /// Classes a unit writes beside its own: the module class of a class whose static
    /// forwarders need its method table, a mirror class of static forwarders.
    pub extra_classes: Vec<(String, Vec<u8>)>,
    /// In link mode, the method table of the module class written last, and the module class
    /// whose static forwarders the class being written takes.
    pub module_table: Vec<(u16, String, String)>,
    pub keep_table: bool,
    pub forward_from: Option<(Rc<str>, Vec<(u16, String, String)>)>,
    pub wide: bool,
    pub overflow: bool,
    /// The `Object` a value class over a type parameter erases to in link mode, told apart by
    /// identity from the `Object` of a generic slot, where the value is boxed.
    vc_object: Rc<str>,
    /// The same for a class a value class conforms to and holds (`class V(val u: U) extends
    /// AnyVal with U`): the underlying value's type, told apart by identity from the class's own
    /// (a parameter declared `U`), where the value is boxed. By the class's binary name.
    vc_marks: FxMap<Rc<str>, Rc<str>>,
    /// The receiver of a value class's parameter selected (`missing[V].u`), which is unboxed
    /// through the box's accessor without the null test (`vc_field_read`).
    vc_select: Option<TExprId>,
    /// The expansions of applied aliases (`alias_expansion`).
    alias_expansions: std::cell::RefCell<FxMap<(AliasId, TList), TypeId>>,
    /// The type parameters whose bounds are being erased, which a bound that names its own
    /// parameter again meets.
    erasing_params: Vec<TParamId>,
}

#[derive(Clone, Copy)]
pub enum Mode<'t> {
    Value(&'t JType),
    /// Return position: every path ends in a return or in a jump of the tail loop.
    Tail,
}

impl<'a> Gen<'a> {
    pub fn new(cx: &'a Cx<'a>) -> Gen<'a> {
        Gen {
            cx,
            errors: Vec::new(),
            erased: Vec::new(),
            sparse: false,
            erased_overlay: FxMap::default(),
            class_rc: vec![None; cx.input.syms.classes.len()],
            mrefs: FxMap::default(),
            erased_arities: FxMap::default(),
            mref_arities: FxMap::default(),
            trait_fields: FxMap::default(),
            cw: ClassWriter::new(0, "x", OBJECT, &[]),
            this_name: Rc::from(""),
            this_class: None,
            is_interface: false,
            abstract_class: false,
            line_file: FileId(u32::MAX),
            line_floor: None,
            line_hold: false,
            pending: Vec::new(),
            lifted: FxMap::default(),
            lifted_defaults: FxMap::default(),
            an: Analysis::default(),
            counter: 0,
            class_captures: Vec::new(),
            code: Code::new(Vec::new()),
            m: MethodCx::default(),
            context: String::new(),
            function_arities: 0b11,
            extra_classes: Vec::new(),
            module_table: Vec::new(),
            keep_table: false,
            forward_from: None,
            wide: false,
            overflow: false,
            vc_object: Rc::from(OBJECT),
            vc_marks: FxMap::default(),
            vc_select: None,
            alias_expansions: Default::default(),
            erasing_params: Vec::new(),
        }
    }

    pub fn unsupported(&mut self, what: &str) {
        self.errors.push(format!("{}: {}", self.context, what));
    }

    /// A class with a method too long for 16-bit jump offsets is emitted again with wide jumps.
    pub fn emit_unit(&mut self, unit: Unit) -> (String, Vec<u8>) {
        self.wide = false;
        let errors = self.errors.len();
        let out = self.emit_unit_once(unit);
        if !self.overflow {
            return out;
        }
        self.errors.truncate(errors);
        self.wide = true;
        self.overflow = false;
        let out = self.emit_unit_once(unit);
        if self.overflow {
            self.unsupported("a method whose code exceeds what the JVM allows");
        }
        self.wide = false;
        self.overflow = false;
        out
    }

    fn emit_unit_once(&mut self, unit: Unit) -> (String, Vec<u8>) {
        self.extra_classes.clear();
        self.reset_class();
        match unit {
            Unit::Class(i) => self.emit_class_unit(i),
            Unit::File(f) => self.emit_file_unit(f),
        }
    }

    pub fn reset_class(&mut self) {
        self.pending.clear();
        self.lifted.clear();
        self.lifted_defaults.clear();
        self.an = Analysis::default();
        self.counter = 0;
        self.class_captures.clear();
    }

    // ---- types ----

    pub fn class_name(&mut self, c: ClassId) -> Rc<str> {
        if let Some(n) = &self.class_rc[c.idx()] {
            return n.clone();
        }
        let n: Rc<str> = Rc::from(self.cx.class_names[c.idx()].as_str());
        self.class_rc[c.idx()] = Some(n.clone());
        n
    }

    pub fn class_type(&mut self, c: ClassId) -> JType {
        JType::L(self.class_name(c))
    }

    pub fn function_type(&mut self, arity: usize) -> JType {
        self.function_arities |= 1u64 << arity.min(63);
        JType::L(Rc::from(format!("scala/Function{}", arity)))
    }

    pub fn erase(&mut self, t: TypeId) -> JType {
        if t == NO_TYPE {
            return JType::object();
        }
        if t.0 >= crate::arena::LOCAL_BASE || self.sparse {
            return self.erase_overlay(t);
        }
        if let Some(Some(j)) = self.erased.get(t.idx()) {
            let j = j.clone();
            if let Some(&used) = self.erased_arities.get(&t) {
                self.function_arities |= used;
            }
            return j;
        }
        let before = std::mem::take(&mut self.function_arities);
        let j = self.erase_uncached(t);
        self.keep_arities(before, |g, used| g.erased_arities.insert(t, used));
        if self.erased.len() <= t.idx() {
            self.erased.resize(t.idx() + 1, None);
        }
        self.erased[t.idx()] = Some(j.clone());
        j
    }

    fn erase_overlay(&mut self, t: TypeId) -> JType {
        if let Some(j) = self.erased_overlay.get(&t) {
            let j = j.clone();
            if let Some(&used) = self.erased_arities.get(&t) {
                self.function_arities |= used;
            }
            return j;
        }
        let before = std::mem::take(&mut self.function_arities);
        let j = self.erase_uncached(t);
        self.keep_arities(before, |g, used| g.erased_arities.insert(t, used));
        self.erased_overlay.insert(t, j.clone());
        j
    }

    /// After a memo's entry was made with the arities taken from `before`: notes the ones past 1
    /// it used with the entry and puts `before` back beside them.
    fn keep_arities(&mut self, before: u64, note: impl FnOnce(&mut Self, u64) -> Option<u64>) {
        let used = self.function_arities & !0b11;
        self.function_arities |= before;
        if used != 0 {
            note(self, used);
        }
    }

    fn erase_uncached(&mut self, t: TypeId) -> JType {
        fn context_function_arity(cx: &Cx, c: ClassId) -> Option<usize> {
            let info = cx.input.syms.class(c);
            let Owner::Package(p) = info.owner else { return None };
            if cx.input.interner.get(cx.input.syms.pkg(p).name) != "scala" {
                return None;
            }
            cx.input.interner.get(info.name).strip_prefix("ContextFunction")?.parse().ok()
        }
        let cx = self.cx;
        match cx.input.types.get(t) {
            Type::Class(c, args) => {
                let info = cx.input.syms.class(c);
                // A context function is a plain function of its arity at run time.
                if let Some(n) = context_function_arity(cx, c) {
                    return self.function_type(n);
                }
                if let Some(name) = special_erasure(cx, c) {
                    return JType::L(Rc::from(name));
                }
                // A `*:` chain is the tuple class of its arity where it ends in one, `Product`
                // where it does not (`Int *: EmptyTuple`, `H *: T`), as scalac's `erasePair`.
                if scala_class(cx, c) == Some("*:") {
                    return match cx.input.types.cons_arity(t, c, &|k| tuple_arity(cx, k)) {
                        Some(n) if n <= 22 => JType::L(Rc::from(format!("scala/Tuple{}", n))),
                        Some(_) => JType::L(Rc::from(TUPLE_XXL)),
                        None => JType::L(Rc::from(PRODUCT)),
                    };
                }
                match info.kind {
                    ClassKind::Builtin => match cx.input.interner.get(info.name) {
                        "Int" => JType::I,
                        "Long" => JType::J,
                        "Double" => JType::D,
                        "Byte" => JType::B,
                        "Short" => JType::S,
                        "Float" => JType::F,
                        "Boolean" => JType::Z,
                        "Char" => JType::C,
                        "Unit" => JType::V,
                        "Array" => {
                            let elem = cx.input.types.items(args).first().copied();
                            self.array_type(elem)
                        }
                        _ => self.class_type(c),
                    },
                    // An opaque type erases to its underlying type; one that is a parameter
                    // (`opaque type D[A] = A`) to the argument.
                    ClassKind::Opaque => match info.underlying {
                        Some(u) => match cx.input.types.get(u) {
                            Type::Param(p) => match info.tparams.iter().position(|&q| q == p).and_then(|i| cx.input.types.items(args).get(i).copied()) {
                                Some(arg) => self.erase(arg),
                                None => self.erase(u),
                            },
                            _ => self.erase(u),
                        },
                        None => JType::object(),
                    },
                    _ if info.value_class => self.vc_erasure(c, args),
                    _ if info.js != JsKind::Scala => JType::object(),
                    _ => self.class_type(c),
                }
            }
            Type::Param(p) => self.erase_param(p),
            Type::AppParam(p, args) => self.erase_applied_param(p, args),
            Type::Var(v) => match cx.input.insts.inst(v) {
                Some(inst) => self.erase(inst),
                None => JType::object(),
            },
            Type::Union(a, b) => self.erase_union(a, b, false, &mut Vec::new()),
            Type::Inter(a, b) => {
                // A value class comes before every other part, as scalac's `ErasedValueType` does,
                // two of them ordered by their classes; the one chosen erases to its underlying type.
                match (self.value_class_of(a), self.value_class_of(b)) {
                    (Some(ca), Some(cb)) => {
                        let (na, nb) = (self.class_type(ca), self.class_type(cb));
                        let first = self.erased_glb_first(&na, &nb);
                        return self.erase(if first { a } else { b });
                    }
                    (Some(_), None) => return self.erase(a),
                    (None, Some(_)) => return self.erase(b),
                    (None, None) => {}
                }
                let (ea, eb) = (self.erase(a), self.erase(b));
                if self.erased_glb_first(&ea, &eb) { ea } else { eb }
            }
            Type::Lit(l) => match cx.input.types.lit_val(l) {
                LitVal::Int(_) => JType::I,
                LitVal::Long(_) => JType::J,
                LitVal::Double(_) => JType::D,
                LitVal::Char(_) => JType::C,
                LitVal::Bool(_) => JType::Z,
                LitVal::Str(_) => JType::L(Rc::from(STRING)),
            },
            // As scalac erases them: a path to its type, an abstract member to its upper
            // bound, a refinement to its parent.
            Type::This(c) => self.class_type(c),
            Type::Term(s) | Type::Select(_, s) => match cx.input.syms.sym(s).sig.as_ref().map(|sig| sig.ret) {
                Some(ret) => self.erase(ret),
                None => JType::object(),
            },
            Type::Member(prefix, name) => match self.member_upper_bound(prefix, name) {
                Some(upper) => self.erase(upper),
                None => JType::object(),
            },
            Type::AppMember(m, _) => self.erase(m),
            Type::Decl(a) => match cx.input.syms.aliases[a.idx()].bounds {
                Some((_, upper)) => self.erase(upper),
                None => JType::object(),
            },
            Type::Lambda(_, body) | Type::Poly(_, body) => self.erase(body),
            Type::Refined(parent, _) => self.erase(parent),
            // A match type the typer left unreduced erases to its bound.
            Type::Match(_, m) => {
                let bound = cx.input.types.match_info(m).bound;
                self.erase(bound)
            }
            // An applied alias erases as its expansion does (dotty's `checkedSuperType`,
            // `TypeErasure.scala` 768, 779), a match type it stands for to its bound over the
            // application's arguments.
            Type::Alias(a, args) => match self.alias_expansion(a, args) {
                Some(x) => self.erase(x),
                None => JType::object(),
            },
            _ => JType::object(),
        }
    }

    /// A type parameter erases as its upper bound does, `Unit` as its box: scalac's
    /// `f[T <: String](t: T): T` is `(String)String`, `f[T <: Unit](t: T): T`
    /// `(BoxedUnit)BoxedUnit`. A bound that names the parameter again where it erases
    /// (`T <: Array[T]`, which scalac refuses) gives `Object` there.
    fn erase_param(&mut self, p: TParamId) -> JType {
        if self.erasing_params.contains(&p) {
            return JType::object();
        }
        self.erasing_params.push(p);
        let upper = self.cx.input.syms.tparam(p).upper;
        let j = self.written_bottom(upper).unwrap_or_else(|| self.storage(upper));
        self.erasing_params.pop();
        j
    }

    /// `F[A]` of a higher-kinded parameter erases as its bound applied to the arguments:
    /// `F[Int]` with `F[X] <: Array[X]` is an `int[]`.
    fn erase_applied_param(&mut self, p: TParamId, args: TList) -> JType {
        if self.erasing_params.contains(&p) {
            return JType::object();
        }
        self.erasing_params.push(p);
        let j = match self.applied_upper(p, args) {
            Applied::Is(t) => self.written_bottom(t).unwrap_or_else(|| self.storage(t)),
            Applied::ArrayOf(e) => self.array_type(Some(e)),
        };
        self.erasing_params.pop();
        j
    }

    /// The upper bound of `F` applied to `args`, as far as the erasure reads it without making
    /// a type: the argument a bound `[X] =>> X` returns, the element of `[X] =>> Array[X]`, the
    /// bound's body otherwise, whose class the arguments do not change.
    fn applied_upper(&self, p: TParamId, args: TList) -> Applied {
        let types = self.cx.input.types;
        let upper = self.cx.input.syms.tparam(p).upper;
        let Type::Lambda(params, body) = types.get(upper) else { return Applied::Is(upper) };
        let arg_of = |t: TypeId| match types.get(t) {
            Type::Param(x) => types.items(params).iter().position(|&q| matches!(types.get(q), Type::Param(y) if y == x)).and_then(|i| types.items(args).get(i).copied()),
            _ => None,
        };
        match types.get(body) {
            Type::Param(_) => arg_of(body).map_or(Applied::Is(body), Applied::Is),
            Type::Class(c, bargs) if self.cx.input.syms.class(c).kind == ClassKind::Builtin && self.cx.input.interner.get(self.cx.input.syms.class(c).name) == "Array" => {
                match types.items(bargs).first().and_then(|&e| arg_of(e)) {
                    Some(e) => Applied::ArrayOf(e),
                    None => Applied::Is(body),
                }
            }
            _ => Applied::Is(body),
        }
    }

    /// `Nothing` and `Null` as a signature writes them (a declared result, value or parameter
    /// type, a bound) erase to scala-library's `Nothing$` and `Null$`, which a scalac caller's
    /// descriptor names; elsewhere, a type the typer inferred, they erase to `Object`, which a
    /// value of the type scalac would have inferred there fits.
    fn written_bottom(&self, t: TypeId) -> Option<JType> {
        let cx = self.cx;
        let which = match cx.input.types.get(t) {
            Type::Nothing => 0,
            Type::Class(c, _) if cx.input.syms.class(c).kind == ClassKind::Builtin => match cx.input.interner.get(cx.input.syms.class(c).name) {
                "Nothing" => 0,
                "Null" => 1,
                _ => return None,
            },
            _ => return None,
        };
        Some(JType::L(Rc::from(super::runtime::ARRAY_MARKERS[which])))
    }

    /// Whether the source of `s` writes its type (`def f: T`, `val x: T`), which `written_bottom`
    /// reads.
    fn type_written(&self, s: SymId) -> bool {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        let Some(d) = info.def else { return false };
        match &cx.input.asts[info.file.0 as usize].def(d).kind {
            crate::ast::DefKind::Val { ty, .. } => ty.is_some(),
            crate::ast::DefKind::Fun(f) => f.ret.is_some(),
            _ => false,
        }
    }

    /// The erasure of `A | B`, scalac's `erasedLub` of the parts' erasures (`Lub`): `Nothing | T`
    /// is `T`'s and `Null | C` is `C`'s for a reference `C`, two arrays of references the array of
    /// their elements' lub, two classes the last of their common base classes up to the first
    /// that no other common one derives from (`erased_lub`), `Object` where the parts erase to
    /// other kinds. `boxed` erases a value class as its class (an array's elements); `seen`
    /// holds the type parameters on the way to the union.
    fn erase_union(&mut self, a: TypeId, b: TypeId, boxed: bool, seen: &mut Vec<TParamId>) -> JType {
        let (la, lb) = (self.lub_part(a, boxed, seen), self.lub_part(b, boxed, seen));
        let l = self.lub(la, lb);
        self.lub_type(&l)
    }

    /// A union's part as `erase_union` reads it.
    fn lub_part(&mut self, t: TypeId, boxed: bool, seen: &mut Vec<TParamId>) -> Lub {
        let cx = self.cx;
        match cx.input.types.get(t) {
            Type::Nothing => Lub::Nothing,
            Type::Class(c, args) => {
                let info = cx.input.syms.class(c);
                if info.kind == ClassKind::Builtin {
                    match cx.input.interner.get(info.name) {
                        "Nothing" => return Lub::Nothing,
                        "Null" => return Lub::Null,
                        "Array" => {
                            return match cx.input.types.items(args).first() {
                                Some(&e) if !self.generic_array_element(e) => Lub::Array(Box::new(self.lub_part(e, true, seen))),
                                _ => Lub::Erased(JType::object()),
                            }
                        }
                        _ => return Lub::Erased(self.lub_erasure(t, boxed)),
                    }
                }
                // A class whose erasure is its own name is one whose bases the lub walks; a value
                // class keeps its class beside its underlying erasure.
                let e = self.lub_erasure(t, boxed);
                if e == self.class_type(c) {
                    Lub::Class(c)
                } else if info.value_class {
                    Lub::Value(c, e)
                } else {
                    Lub::Erased(e)
                }
            }
            Type::Param(p) | Type::AppParam(p, _) => {
                if seen.contains(&p) {
                    return Lub::Erased(JType::object());
                }
                seen.push(p);
                let l = match cx.input.types.get(t) {
                    Type::AppParam(_, args) => match self.applied_upper(p, args) {
                        Applied::Is(u) => self.lub_part(u, boxed, seen),
                        Applied::ArrayOf(e) if !self.generic_array_element(e) => Lub::Array(Box::new(self.lub_part(e, true, seen))),
                        Applied::ArrayOf(_) => Lub::Erased(JType::object()),
                    },
                    _ => self.lub_part(cx.input.syms.tparam(p).upper, boxed, seen),
                };
                seen.pop();
                l
            }
            Type::Union(a, b) => {
                let (la, lb) = (self.lub_part(a, boxed, seen), self.lub_part(b, boxed, seen));
                self.lub(la, lb)
            }
            // The side the erasure of the intersection takes: a value class's, else scalac's
            // `erasedGlb` of the two.
            Type::Inter(a, b) => {
                if !boxed {
                    if let Some(c) = self.value_class_of(t) {
                        return Lub::Value(c, self.erase(t));
                    }
                }
                let (la, lb) = (self.lub_part(a, boxed, seen), self.lub_part(b, boxed, seen));
                let (ja, jb) = (self.lub_type(&la), self.lub_type(&lb));
                if self.erased_glb_first(&ja, &jb) { la } else { lb }
            }
            Type::Var(v) => match cx.input.insts.inst(v) {
                Some(inst) => self.lub_part(inst, boxed, seen),
                None => Lub::Erased(JType::object()),
            },
            Type::Term(s) | Type::Select(_, s) => match cx.input.syms.sym(s).sig.as_ref().map(|sig| sig.ret) {
                Some(ret) => self.lub_part(ret, boxed, seen),
                None => Lub::Erased(JType::object()),
            },
            Type::Refined(parent, _) => self.lub_part(parent, boxed, seen),
            Type::Member(..) | Type::Decl(_) => match self.abstract_upper(t) {
                Some(upper) => self.lub_part(upper, boxed, seen),
                None => Lub::Erased(JType::object()),
            },
            _ => Lub::Erased(self.lub_erasure(t, boxed)),
        }
    }

    /// A part's own erasure, `Unit` as its box.
    fn lub_erasure(&mut self, t: TypeId, boxed: bool) -> JType {
        match if boxed { self.erase_boxed_vc(t) } else { self.erase(t) } {
            JType::V => JType::L(Rc::from(BOXED_UNIT)),
            j => j,
        }
    }

    fn lub(&mut self, x: Lub, y: Lub) -> Lub {
        let reference = |l: &Lub| !matches!(l, Lub::Erased(j) if !j.is_ref()) && !matches!(l, Lub::Value(..));
        match (&x, &y) {
            (Lub::Nothing, _) => return y,
            (_, Lub::Nothing) => return x,
            (Lub::Null, y) if reference(y) => return y.clone(),
            (x, Lub::Null) if reference(x) => return x.clone(),
            _ => {}
        }
        // Two parts that erase alike (`Show[Int] | D[Show[Int]]` of an opaque `D[A] = A`); never
        // two value classes, which scalac's lub takes to `Object` even where they are one class
        // (`V | V`, `V | W` over one `Int`).
        if matches!(x, Lub::Value(..)) || matches!(y, Lub::Value(..)) {
            return Lub::Erased(JType::object());
        }
        if x == y || self.lub_type(&x) == self.lub_type(&y) {
            return x;
        }
        match (x, y) {
            (Lub::Array(ex), Lub::Array(ey)) if reference(&ex) && reference(&ey) => Lub::Array(Box::new(self.lub(*ex, *ey))),
            (Lub::Class(a), Lub::Class(b)) => self.erased_lub(a, b).map_or(Lub::Erased(JType::object()), Lub::Class),
            _ => Lub::Erased(JType::object()),
        }
    }

    fn lub_type(&mut self, l: &Lub) -> JType {
        match l {
            Lub::Class(c) => self.class_type(*c),
            Lub::Array(e) => {
                let e = match **e {
                    Lub::Nothing => self.array_marker(0),
                    Lub::Null => self.array_marker(1),
                    ref e => self.lub_type(e),
                };
                let mut d = String::from("[");
                e.desc(&mut d);
                JType::L(Rc::from(d.as_str()))
            }
            Lub::Nothing | Lub::Null => JType::object(),
            Lub::Value(_, j) | Lub::Erased(j) => j.clone(),
        }
    }

    /// scalac's `erasedLub` of two classes: of `a`'s base classes in linearisation order those
    /// `b` derives from, the classes up to the first trait and that trait (its `takeUntil`); of
    /// those, the last that no other derives from (`TreeSeqMap | VectorMap` is the trait
    /// `StrictOptimizedMapOps`, `PA | PB` of two subclasses of `P0` is `P0`).
    fn erased_lub(&self, a: ClassId, b: ClassId) -> Option<ClassId> {
        let syms = self.cx.input.syms;
        let derives = |x: ClassId, y: ClassId| x == y || syms.class(x).base_types.iter().any(|&(k, _)| k == y);
        let mut candidates: Vec<ClassId> = Vec::new();
        for &(k, _) in &syms.class(a).base_types {
            if !derives(b, k) {
                continue;
            }
            candidates.push(k);
            if syms.class(k).kind == ClassKind::Trait {
                break;
            }
        }
        candidates.iter().copied().filter(|&cand| candidates.iter().all(|&x| x == cand || !derives(x, cand))).last()
    }

    /// The upper bound of a type parameter or of an abstract type member.
    fn abstract_upper(&mut self, t: TypeId) -> Option<TypeId> {
        let cx = self.cx;
        match cx.input.types.get(t) {
            Type::Param(p) | Type::AppParam(p, _) => Some(cx.input.syms.tparam(p).upper),
            Type::Member(prefix, name) => self.member_upper_bound(prefix, name),
            Type::Decl(a) => cx.input.syms.aliases[a.idx()].bounds.map(|(_, upper)| upper),
            _ => None,
        }
    }

    /// The kind of JVM array that holds every value of `t`, scalac's `arrayUpperBound`: the
    /// primitive's for a primitive type, `Object` for a reference type, none for a type whose
    /// values may be either (`Any`, `AnyVal`, `Matchable`, `Singleton`, `Int | String`).
    fn array_bound(&mut self, t: TypeId) -> Option<JType> {
        let cx = self.cx;
        match cx.input.types.get(t) {
            Type::Any => None,
            Type::Nothing | Type::This(_) => Some(JType::object()),
            Type::Class(c, args) => {
                let info = cx.input.syms.class(c);
                match info.kind {
                    ClassKind::Builtin => match cx.input.interner.get(info.name) {
                        "AnyVal" => None,
                        "Int" | "Long" | "Double" | "Byte" | "Short" | "Float" | "Boolean" | "Char" | "Unit" => Some(self.erase(t)),
                        _ => Some(JType::object()),
                    },
                    ClassKind::Opaque => {
                        let u = info.underlying?;
                        match cx.input.types.get(u) {
                            Type::Param(p) => match info.tparams.iter().position(|&q| q == p).and_then(|i| cx.input.types.items(args).get(i).copied()) {
                                Some(arg) => self.array_bound(arg),
                                None => self.array_bound(u),
                            },
                            _ => self.array_bound(u),
                        }
                    }
                    _ if matches!(scala_class(cx, c), Some("Matchable" | "Singleton")) => None,
                    _ => Some(JType::object()),
                }
            }
            Type::Lit(_) => Some(self.erase(t)),
            Type::Param(p) | Type::AppParam(p, _) => {
                if self.erasing_params.contains(&p) {
                    return None;
                }
                self.erasing_params.push(p);
                let b = match cx.input.types.get(t) {
                    Type::AppParam(_, args) => match self.applied_upper(p, args) {
                        Applied::Is(u) => self.array_bound(u),
                        Applied::ArrayOf(_) => Some(JType::object()),
                    },
                    _ => self.array_bound(cx.input.syms.tparam(p).upper),
                };
                self.erasing_params.pop();
                b
            }
            Type::Member(..) | Type::Decl(_) => {
                let upper = self.abstract_upper(t)?;
                self.array_bound(upper)
            }
            Type::Term(s) | Type::Select(_, s) => {
                let ret = cx.input.syms.sym(s).sig.as_ref()?.ret;
                self.array_bound(ret)
            }
            Type::Refined(parent, _) | Type::Lambda(_, parent) => self.array_bound(parent),
            Type::Alias(a, args) => {
                let x = self.alias_expansion(a, args)?;
                self.array_bound(x)
            }
            Type::Var(v) => {
                let inst = cx.input.insts.inst(v)?;
                self.array_bound(inst)
            }
            Type::Inter(a, b) => {
                let (ba, bb) = (self.array_bound(a), self.array_bound(b));
                if ba == bb { ba } else { ba.or(bb) }
            }
            Type::Union(a, b) => {
                let (ba, bb) = (self.array_bound(a), self.array_bound(b));
                if ba == bb { ba } else { None }
            }
            _ => None,
        }
    }

    /// Whether `Array[t]` erases to `Object`, scalac's `isGenericArrayElement`: `t` is abstract
    /// and its bound fits no one kind of JVM array, or an intersection of such types, or a
    /// union with one.
    fn generic_array_element(&mut self, t: TypeId) -> bool {
        let cx = self.cx;
        match cx.input.types.get(t) {
            Type::Param(_) | Type::AppParam(..) | Type::Member(..) | Type::Decl(_) => self.array_bound(t).is_none(),
            Type::Inter(a, b) => self.generic_array_element(a) && self.generic_array_element(b),
            Type::Union(a, b) => self.generic_array_element(a) || self.generic_array_element(b),
            Type::Var(v) => cx.input.insts.inst(v).map_or(true, |inst| self.generic_array_element(inst)),
            Type::Alias(a, args) => self.alias_expansion(a, args).map_or(true, |x| self.generic_array_element(x)),
            _ => false,
        }
    }

    /// The erasure of a type where a value class is its class, not its underlying value
    /// (scalac's erasure with `semiEraseVCs` off): the elements of an array, the underlying
    /// value of a value class over a type parameter (`W[V]` holds a `V`).
    fn erase_boxed_vc(&mut self, t: TypeId) -> JType {
        self.erase_boxed_vc_in(t, &mut Vec::new())
    }

    fn erase_boxed_vc_in(&mut self, t: TypeId, seen: &mut Vec<TParamId>) -> JType {
        let cx = self.cx;
        let mut t = t;
        loop {
            match cx.input.types.get(t) {
                Type::Class(c, _) if cx.input.syms.class(c).value_class => return self.class_type(c),
                Type::Param(p) | Type::AppParam(p, _) if !seen.contains(&p) => {
                    seen.push(p);
                    t = cx.input.syms.tparam(p).upper;
                }
                Type::Var(v) => match cx.input.insts.inst(v) {
                    Some(inst) => t = inst,
                    None => return JType::object(),
                },
                Type::Alias(a, args) => match self.alias_expansion(a, args) {
                    Some(x) => t = x,
                    None => return JType::object(),
                },
                Type::Inter(a, b) => {
                    let at = seen.len();
                    let ea = self.erase_boxed_vc_in(a, seen);
                    seen.truncate(at);
                    let eb = self.erase_boxed_vc_in(b, seen);
                    return if self.erased_glb_first(&ea, &eb) { ea } else { eb };
                }
                Type::Union(a, b) => return self.erase_union(a, b, true, seen),
                _ => return self.storage(t),
            }
        }
    }

    /// The type alias `a` applied to `args` stands for, its right-hand side with the arguments
    /// for its parameters (`Types::subst`), so that an array's element, a value class's argument
    /// and a match type's bound are classified as the expansion (`TypeErasure.scala` 779, 901,
    /// 927-935). None for an abstract member and for an expansion that is the application again.
    pub(super) fn alias_expansion(&self, a: AliasId, args: TList) -> Option<TypeId> {
        if let Some(&x) = self.alias_expansions.borrow().get(&(a, args)) {
            return Some(x);
        }
        let types = self.cx.input.types;
        let info = &self.cx.input.syms.aliases[a.idx()];
        // An abstract type member has bounds, no right-hand side (`Type::Decl` erases it).
        if info.is_abstract() {
            return None;
        }
        // The right-hand side over exactly the alias's parameters is stored as a lambda over them.
        let body = match types.get(info.rhs) {
            Type::Lambda(ps, b)
                if types.items(ps).len() == info.tparams.len()
                    && types.items(ps).iter().zip(&info.tparams).all(|(&p, &q)| matches!(types.get(p), Type::Param(r) if r == q)) =>
            {
                b
            }
            _ => info.rhs,
        };
        let items = types.items(args);
        let s: Subst = info.tparams.iter().copied().zip(items.iter().copied()).collect();
        let mut x = types.subst(body, &s);
        // A curried alias (`type F[A] = [B] =>> B`, `F[String][Int]`) takes the rest of the arguments
        // in its lambdas, in order.
        let mut rest = items.get(info.tparams.len()..).unwrap_or(&[]);
        while !rest.is_empty() {
            let Type::Lambda(ps, b) = types.get(x) else { break };
            let ps = types.items(ps);
            let s: Subst = ps.iter().zip(rest).filter_map(|(&p, &a)| match types.get(p) {
                Type::Param(q) => Some((q, a)),
                _ => None,
            }).collect();
            x = types.subst(b, &s);
            rest = &rest[ps.len().min(rest.len())..];
        }
        if matches!(types.get(x), Type::Alias(b, bargs) if b == a && bargs == args) {
            return None;
        }
        self.alias_expansions.borrow_mut().insert((a, args), x);
        Some(x)
    }

    /// The erasure of the value class `c` applied to `args`, scalac's `eraseDerivedValueClass`:
    /// the underlying type as the arguments instantiate it (`W[String]` for `W[A](value: A)` is
    /// a `String`), a primitive boxed where the declared underlying type is a type parameter
    /// (`W[Int]` is an `Integer`), and an array's erasure as the class declares it.
    /// The result is marked as `vc_marked` says.
    fn vc_erasure(&mut self, c: ClassId, args: TList) -> JType {
        let j = self.vc_erasure_plain(c, args);
        self.vc_marked(c, j)
    }

    fn vc_erasure_plain(&mut self, c: ClassId, args: TList) -> JType {
        let cx = self.cx;
        let syms = cx.input.syms;
        let info = syms.class(c);
        let declared = info.ctor.first().and_then(|cl| cl.params.first()).and_then(|p| syms.sym(p.sym).sig.as_ref().map(|sig| sig.ret));
        let arg_of = |p: TParamId| info.tparams.iter().position(|&q| q == p).and_then(|i| cx.input.types.items(args).get(i).copied());
        let arg = match declared.map(|d| cx.input.types.get(d)) {
            Some(Type::Param(p)) => arg_of(p),
            // `H[F[_], A](value: F[A])`: `H[List, Int]` holds a `List`, the class the argument names.
            Some(Type::AppParam(p, applied)) => {
                // `F[A]`'s arguments as the class's arguments instantiate them.
                let applied: Vec<TypeId> = cx.input.types.items(applied).iter().map(|&x| match cx.input.types.get(x) {
                    Type::Param(q) => arg_of(q).unwrap_or(x),
                    _ => x,
                }).collect();
                match arg_of(p).map(|a| cx.input.types.get(a)) {
                    Some(Type::Ctor(k)) => {
                        let info = cx.input.syms.class(k);
                        let array = info.kind == ClassKind::Builtin && cx.input.interner.get(info.name) == "Array";
                        return match special_erasure(cx, k) {
                            Some(name) => JType::L(Rc::from(name)),
                            None if array => self.array_type(applied.first().copied()),
                            None if info.kind == ClassKind::Opaque || info.kind == ClassKind::Builtin || info.js != JsKind::Scala => self.vc_underlying(c),
                            None => self.class_type(k),
                        };
                    }
                    // `[X] =>> X` holds its argument; another body's erasure is its class's.
                    Some(Type::Lambda(params, body)) => match cx.input.types.get(body) {
                        Type::Param(x) => cx.input.types.items(params).iter().position(|&q| matches!(cx.input.types.get(q), Type::Param(y) if y == x)).and_then(|i| applied.get(i).copied()),
                        _ => Some(body),
                    },
                    _ => None,
                }
            }
            _ => None,
        };
        let Some(arg) = arg else { return self.vc_underlying(c) };
        match self.erase_boxed_vc(arg) {
            j if j.is_object() => JType::L(self.vc_object.clone()),
            j if !j.is_ref() => JType::L(Rc::from(j.box_class())),
            j => j,
        }
    }

    /// Whether the erasure of `A & B` is `A`'s, by scalac's order (`TypeErasure.erasedGlb`): an
    /// array, then a primitive, then a class before a trait, then a subclass before its
    /// superclass, then the first by its full name.
    fn erased_glb_first(&self, a: &JType, b: &JType) -> bool {
        if a == b {
            return true;
        }
        let array = |t: &JType| match t {
            JType::L(n) => n.strip_prefix('[').map(desc_type),
            _ => None,
        };
        match (array(a), array(b)) {
            (Some(ea), Some(eb)) => return self.erased_glb_first(&ea, &eb),
            (Some(_), None) => return true,
            (None, Some(_)) => return false,
            (None, None) => {}
        }
        let (JType::L(x), JType::L(y)) = (a, b) else {
            return match (a.is_ref(), b.is_ref()) {
                (false, true) => true,
                (true, false) => false,
                _ => scala_name_of(a) <= scala_name_of(b),
            };
        };
        let cx = self.cx;
        let class = |n: &str| if n == OBJECT { None } else { cx.class_named(n) };
        let (cx_, cy) = (class(x), class(y));
        let interface = |c: Option<ClassId>| c.map_or(false, |c| cx.is_interface(c));
        let (ix, iy) = (interface(cx_), interface(cy));
        if ix != iy {
            return !ix;
        }
        let derives = |sub: Option<ClassId>, sup: Option<ClassId>, sup_name: &str| {
            sup_name == OBJECT || matches!((sub, sup), (Some(s), Some(p)) if cx.input.syms.class(s).base_types.iter().any(|&(b, _)| b == p))
        };
        if derives(cx_, cy, y) {
            return true;
        }
        if derives(cy, cx_, x) {
            return false;
        }
        self.scala_full_name(cx_, x) <= self.scala_full_name(cy, y)
    }

    /// A class's full name as scalac's `fullName` spells it, its source names joined by dots, an
    /// object's (a module class's) with its `$`, which orders the parts of an intersection that
    /// nothing else orders.
    fn scala_full_name(&self, c: Option<ClassId>, binary: &str) -> String {
        let Some(c) = c else { return binary.replace('/', ".") };
        let syms = self.cx.input.syms;
        let interner = self.cx.input.interner;
        let part = |c: ClassId| {
            let info = syms.class(c);
            let name = interner.get(info.name);
            if info.kind == ClassKind::Object { format!("{}$", name) } else { name.to_string() }
        };
        let mut parts = vec![part(c)];
        let mut owner = syms.class(c).owner;
        while let Owner::Class(o) = owner {
            parts.push(part(o));
            owner = syms.class(o).owner;
        }
        let mut pkg = match owner {
            Owner::Package(p) => Some(p),
            _ => None,
        };
        while let Some(p) = pkg {
            let info = syms.pkg(p);
            if info.parent.is_none() {
                break;
            }
            parts.push(interner.get(info.name).to_string());
            pkg = info.parent;
        }
        parts.reverse();
        parts.join(".")
    }

    /// The declared upper bound of the type member `name` of the class the prefix is an
    /// instance of; the bound is in terms of that class, which erasure does not mind.
    fn member_upper_bound(&mut self, prefix: TypeId, name: Name) -> Option<TypeId> {
        let cx = self.cx;
        let mut t = prefix;
        let c = loop {
            match cx.input.types.get(t) {
                Type::This(c) | Type::Class(c, _) => break c,
                Type::Term(s) | Type::Select(_, s) => t = cx.input.syms.sym(s).sig.as_ref()?.ret,
                Type::Refined(p, _) => t = p,
                Type::Var(v) => t = cx.input.insts.inst(v)?,
                Type::Inter(a, _) => t = a,
                _ => return None,
            }
        };
        for &(b, _) in &cx.input.syms.class(c).base_types {
            if let Some(&a) = cx.input.syms.class(b).type_aliases.get(&name) {
                let info = &cx.input.syms.aliases[a.idx()];
                return Some(match info.bounds {
                    Some((_, upper)) => upper,
                    None => info.rhs,
                });
            }
        }
        None
    }

    // ---- JVM arrays ----

    /// `Array[T]` as scalac erases it: a JVM array of the element's erasure, a value class's
    /// elements boxed (`[LMeters;`), and `Object` where the element is a type parameter,
    /// `Any` or an abstract type, whose arrays may be of any kind.
    fn array_type(&mut self, elem: Option<TypeId>) -> JType {
        let Some(elem) = elem else { return JType::object() };
        let cx = self.cx;
        let mut t = elem;
        loop {
            match cx.input.types.get(t) {
                Type::Var(v) => match cx.input.insts.inst(v) {
                    Some(inst) => t = inst,
                    None => return JType::object(),
                },
                // `Array[Id[Int]]` is an `int[]`, `Array[Id[V]]` a `V[]`.
                Type::Alias(a, args) => match self.alias_expansion(a, args) {
                    Some(x) => t = x,
                    None => return JType::object(),
                },
                _ => break,
            }
        }
        // A singleton's array holds what the path's type does.
        match cx.input.types.get(t) {
            Type::Term(s) | Type::Select(_, s) => {
                if let Some(under) = cx.input.syms.sym(s).sig.as_ref().map(|sig| sig.ret) {
                    return self.array_type(Some(under));
                }
            }
            Type::This(c) => {
                let e = self.class_type(c);
                let mut d = String::from("[");
                e.desc(&mut d);
                return JType::L(Rc::from(d.as_str()));
            }
            _ => {}
        }
        if self.generic_array_element(t) {
            return JType::object();
        }
        let e = match cx.input.types.get(t) {
            Type::Any => JType::object(),
            Type::Nothing => self.array_marker(0),
            // A bound that fits one kind of array is the element's: `Array[T <: String]` is a
            // `String[]`, `Array[T <: Int]` an `int[]`, `Array[T <: V]` a `V[]`.
            Type::Param(p) | Type::AppParam(p, _) => {
                if self.erasing_params.contains(&p) {
                    return JType::object();
                }
                self.erasing_params.push(p);
                let a = match cx.input.types.get(t) {
                    Type::AppParam(_, args) => match self.applied_upper(p, args) {
                        Applied::Is(u) => self.array_type(Some(u)),
                        Applied::ArrayOf(e) => {
                            let inner = self.array_type(Some(e));
                            let mut d = String::from("[");
                            inner.desc(&mut d);
                            JType::L(Rc::from(d.as_str()))
                        }
                    },
                    _ => self.array_type(Some(cx.input.syms.tparam(p).upper)),
                };
                self.erasing_params.pop();
                return a;
            }
            Type::Member(..) | Type::Decl(_) => match self.abstract_upper(t) {
                Some(upper) => return self.array_type(Some(upper)),
                None => return JType::object(),
            },
            Type::Inter(..) | Type::Union(..) => self.erase_boxed_vc(t),
            Type::Class(c, _) if cx.input.syms.class(c).value_class => self.class_type(c),
            Type::Class(c, _) if cx.input.syms.class(c).kind == ClassKind::Builtin => match cx.input.interner.get(cx.input.syms.class(c).name) {
                "Null" => self.array_marker(1),
                "Nothing" => self.array_marker(0),
                _ => self.storage(t),
            },
            Type::Class(..) | Type::Lit(_) => self.storage(t),
            _ => return JType::object(),
        };
        let mut d = String::from("[");
        e.desc(&mut d);
        JType::L(Rc::from(d.as_str()))
    }

    /// The class scalac erases `Nothing` (0) or `Null` (1) to, scala-library's, which is also
    /// the component of their arrays.
    fn array_marker(&mut self, which: usize) -> JType {
        JType::L(Rc::from(super::runtime::ARRAY_MARKERS[which]))
    }

    /// An operation on an array whose kind only the value knows, its operands on the stack:
    /// `ScalaRunTime`'s method of the name.
    pub fn erased_array_call(&mut self, name: &str, params: &[JType], ret: JType) -> JType {
        self.invoke(Invoke::Static, SCALA_RUNTIME, false, name, params, &ret);
        ret
    }

    pub fn array_element(t: &JType) -> Option<JType> {
        match t {
            JType::L(n) if n.starts_with('[') => Some(parse_type(&n[1..]).0),
            _ => None,
        }
    }

    /// `array(i)` with the array and the index on the stack: the load instruction of a JVM
    /// array's kind, `ScalaRunTime.array_apply` on an erased one.
    pub fn array_load(&mut self, array: &JType) -> JType {
        let Some(elem) = Self::array_element(array) else {
            return self.erased_array_call("array_apply", &[JType::object(), JType::I], JType::object());
        };
        let opcode = match elem {
            JType::I => op::IALOAD,
            JType::J => op::LALOAD,
            JType::F => op::FALOAD,
            JType::D => op::DALOAD,
            JType::Z | JType::B => op::BALOAD,
            JType::C => op::CALOAD,
            JType::S => op::SALOAD,
            _ => op::AALOAD,
        };
        self.code.op(opcode);
        self.code.popn(2);
        let vt = self.vt(&elem);
        self.code.push(vt);
        elem
    }

    /// `array(i) = v` with the array, the index and the value (of type `value`) on the stack.
    pub fn array_store(&mut self, array: &JType, value: &JType) {
        let Some(elem) = Self::array_element(array) else {
            let object = JType::object();
            self.adapt(value, &object);
            self.erased_array_call("array_update", &[object.clone(), JType::I, object], JType::V);
            return;
        };
        self.adapt(value, &elem);
        let opcode = match elem {
            JType::I => op::IASTORE,
            JType::J => op::LASTORE,
            JType::F => op::FASTORE,
            JType::D => op::DASTORE,
            JType::Z | JType::B => op::BASTORE,
            JType::C => op::CASTORE,
            JType::S => op::SASTORE,
            _ => op::AASTORE,
        };
        self.code.op(opcode);
        self.code.popn(3);
    }

    pub fn array_length(&mut self, array: &JType) {
        if Self::array_element(array).is_none() {
            self.erased_array_call("array_length", &[JType::object()], JType::I);
            return;
        }
        self.code.op(op::ARRAYLENGTH);
        self.code.pop();
        self.code.push(VT::Int);
    }

    pub fn array_clone(&mut self, array: &JType) -> JType {
        if Self::array_element(array).is_none() {
            return self.erased_array_call("array_clone", &[JType::object()], JType::object());
        }
        let JType::L(name) = array else { return array.clone() };
        let object = JType::object();
        self.invoke_desc(Invoke::Virtual, name, false, "clone", "()Ljava/lang/Object;", 0, &object);
        self.checkcast(name);
        array.clone()
    }

    /// A new array of type `array` whose length is on the stack.
    pub fn new_array(&mut self, array: &JType) {
        let Some(elem) = Self::array_element(array) else {
            self.unsupported("a new array of an element type only a ClassTag knows");
            return;
        };
        let atype = match elem {
            JType::Z => Some(4),
            JType::C => Some(5),
            JType::F => Some(6),
            JType::D => Some(7),
            JType::B => Some(8),
            JType::S => Some(9),
            JType::I => Some(10),
            JType::J => Some(11),
            _ => None,
        };
        match (atype, &elem) {
            (Some(code), _) => self.code.op_u8(op::NEWARRAY, code),
            (None, JType::L(n)) => {
                let index = self.cw.cp.class(n);
                self.code.op_u16(op::ANEWARRAY, index);
            }
            _ => {}
        }
        self.code.pop();
        let vt = self.vt(array);
        self.code.push(vt);
    }

    // ---- value classes in link mode ----

    /// The erasure of a value class in link mode, as scalac's: the erasure of its underlying
    /// type, the type of its one parameter.
    pub fn vc_underlying(&mut self, c: ClassId) -> JType {
        let syms = self.cx.input.syms;
        let Some(field) = syms.class(c).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym) else { return JType::object() };
        let Some(ret) = syms.sym(field).sig.as_ref().map(|sig| sig.ret) else { return JType::object() };
        let t = self.storage(ret);
        self.vc_marked(c, t)
    }

    pub fn is_vc_object(&self, t: &JType) -> bool {
        matches!(t, JType::L(n) if Rc::ptr_eq(n, &self.vc_object))
    }

    /// Whether `t` is the marked type of a value class's underlying value (`vc_object`,
    /// `vc_marks`), which a reference of the same descriptor is not.
    pub fn is_vc_mark(&self, t: &JType) -> bool {
        match t {
            JType::L(n) => Rc::ptr_eq(n, &self.vc_object) || self.vc_marks.get(n).map_or(false, |m| Rc::ptr_eq(m, n)),
            _ => false,
        }
    }

    /// `u` without the mark of a value class's underlying value (`vc_marked`): the ordinary type of
    /// that descriptor, which a parameter declared with the type takes.
    fn vc_unmarked(&self, u: &JType) -> JType {
        match u {
            JType::L(n) if self.is_vc_mark(u) => JType::L(Rc::from(&**n)),
            _ => u.clone(),
        }
    }

    /// The erasure `j` of the underlying value of value class `c`, marked where a reference of
    /// that descriptor could be the box: `Object`, or a class `c` conforms to. dotty keeps the
    /// two apart as `ErasedValueType` and the class (`Erasure.scala` 385-406).
    fn vc_marked(&mut self, c: ClassId, j: JType) -> JType {
        let JType::L(n) = &j else { return j };
        if self.is_vc_mark(&j) {
            return j;
        }
        if j.is_object() {
            return JType::L(self.vc_object.clone());
        }
        if n.starts_with('[') || !self.vc_conforms(c, n) {
            return j;
        }
        let n = n.clone();
        JType::L(self.vc_marks.entry(n.clone()).or_insert_with(|| Rc::from(&*n)).clone())
    }

    /// The value class a type is an instance of.
    pub fn value_class_of(&self, t: TypeId) -> Option<ClassId> {
        self.value_class_type(t).map(|(c, _)| c)
    }

    /// The value class a type is an instance of and its type arguments; a type parameter
    /// bounded by one is an instance of it, which erases as the class does.
    pub(super) fn value_class_type(&self, t: TypeId) -> Option<(ClassId, TList)> {
        self.value_class_type_in(t, &mut Vec::new())
    }

    fn value_class_type_in(&self, t: TypeId, seen: &mut Vec<TParamId>) -> Option<(ClassId, TList)> {
        let cx = self.cx;
        let mut t = t;
        loop {
            match cx.input.types.get(t) {
                Type::Class(c, args) if cx.input.syms.class(c).value_class => return Some((c, args)),
                Type::Var(v) => t = cx.input.insts.inst(v)?,
                Type::Alias(a, args) => t = self.alias_expansion(a, args)?,
                Type::Param(p) | Type::AppParam(p, _) if !seen.contains(&p) => {
                    seen.push(p);
                    t = cx.input.syms.tparam(p).upper;
                }
                // A pattern's binder: the scrutinee's type and the tested one (`AnyVal & Meters`).
                Type::Inter(a, b) => {
                    let at = seen.len();
                    if let Some(found) = self.value_class_type_in(a, seen) {
                        return Some(found);
                    }
                    seen.truncate(at);
                    return self.value_class_type_in(b, seen);
                }
                _ => return None,
            }
        }
    }

    /// The value class the value of `e` is an instance of: its recorded type's, or where the
    /// typer recorded a wider type over the node (an ascription or an upcast, `(v: Any)`,
    /// `v.asInstanceOf[Any]`, which keep no node of their own) the node's own, as dotty's
    /// `Typed` keeps its expression's type for `adaptToType` to box (`Erasure.scala` 594-604).
    pub(super) fn expr_value_class(&self, e: TExprId) -> Option<(ClassId, TList)> {
        if let Some(found) = self.recorded_type(e).and_then(|rt| self.value_class_type(rt)) {
            return Some(found);
        }
        let cx = self.cx;
        let declared = |s: SymId| {
            let info = cx.input.syms.sym(s);
            if info.by_name {
                return None;
            }
            info.sig.as_ref().and_then(|sig| self.value_class_type(sig.ret))
        };
        match cx.input.prog.expr(e) {
            TExpr::New(c, _) if cx.input.syms.class(c).value_class => Some((c, EMPTY_LIST)),
            // `this` in a value class's `m$extension`, the underlying value.
            TExpr::This if self.m.this_underlying.is_some() => self.this_class.filter(|&c| cx.input.syms.class(c).value_class).map(|c| (c, EMPTY_LIST)),
            TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) | TExpr::CallStatic(s, _) | TExpr::CallMethod(_, s, _) => declared(s),
            _ => None,
        }
    }

    /// Whether a value of erased type `t` is the underlying value of a value class that
    /// erases to `u`, rather than its box; the `Object` of the class's own erasure is one where
    /// its arguments make it another reference (`W[String]` is a `String`).
    fn holds_underlying(&self, t: &JType, u: &JType) -> bool {
        // A marked type is a value class's underlying value, whichever mark: the declaration's
        // `Object` of `V[A]` holds `V[U]`'s marked `U`, and the marked bound `U` of
        // `V[A <: U]` holds `V[R]`'s `R`.
        if self.is_vc_mark(u) {
            self.is_vc_mark(t)
        } else {
            t == u || !t.is_ref() || u.is_ref() && self.is_vc_mark(t)
        }
    }

    /// Boxes the underlying value on the stack in its value class.
    pub fn vc_box(&mut self, c: ClassId, u: &JType) {
        let slot = self.store_new(u);
        let name = self.class_name(c);
        self.new_object(&name);
        self.load(slot, u);
        let desc = self
            .cx
            .class_files
            .get(&c)
            .and_then(|cf| cf.methods.iter().find(|m| m.name == "<init>" && parse_method_desc(&m.descriptor).0.len() == 1))
            .map(|m| m.descriptor.clone())
            .unwrap_or_else(|| {
                let declared = self.vc_underlying(c);
                method_desc(std::slice::from_ref(&declared), &JType::V)
            });
        let param = parse_method_desc(&desc).0.remove(0);
        self.adapt(u, &param);
        self.invoke_desc(Invoke::Special, &name, false, "<init>", &desc, 1, &JType::V);
    }

    /// The underlying value of the value class box on the stack, through the accessor of its
    /// field (`self()`, or the expanded `scala$collection$StringOps$$s()` of a private one).
    pub fn vc_unbox(&mut self, c: ClassId, u: &JType) {
        let name = self.class_name(c);
        let accessor = match self.cx.class_files.get(&c) {
            Some(cf) => {
                let field = cf.fields.iter().find(|f| f.access & ACC_STATIC == 0);
                field.and_then(|f| {
                    let desc = format!("(){}", f.descriptor);
                    let expanded = format!("$${}", f.name);
                    cf.methods.iter().find(|m| m.access & ACC_STATIC == 0 && m.descriptor == desc && (m.name == f.name || m.name.ends_with(&expanded))).map(|m| (m.name.clone(), desc))
                })
            }
            None => {
                let syms = self.cx.input.syms;
                let field = syms.class(c).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym);
                field.map(|f| (self.field_name(f), method_desc(&[], u)))
            }
        };
        let Some((accessor, desc)) = accessor else {
            self.unsupported("the underlying value of a value class without an accessor");
            return;
        };
        let (_, ret) = parse_method_desc(&desc);
        self.invoke_desc(Invoke::Virtual, &name, false, &accessor, &desc, 0, &ret);
        self.adapt(&ret, u);
    }

    /// Brings a value to the type a symbol is declared with; where that is a value class, a
    /// generic value (a lambda's parameter, a pattern's binding) arrives as its box.
    pub fn adapt_to_sym(&mut self, from: &JType, s: SymId) -> JType {
        let declared = self.sym_type(s);
        let vc = self.cx.input.syms.sym(s).sig.as_ref().and_then(|sig| self.value_class_type(sig.ret));
        match vc {
            Some((c, args)) if !self.cx.input.syms.sym(s).by_name => self.adapt_vc(c, args, from.clone(), &declared),
            _ => self.adapt(from, &declared),
        }
        declared
    }

    /// Brings the value of `s`, stored as `from`, to `want`: a value class's underlying value
    /// is boxed where a reference is wanted (a case class's `productElement` of a field of one).
    pub fn adapt_from_sym(&mut self, s: SymId, from: &JType, want: &JType) {
        let info = self.cx.input.syms.sym(s);
        match info.sig.as_ref().and_then(|sig| self.value_class_type(sig.ret)) {
            Some((c, args)) if !info.by_name => self.adapt_vc(c, args, from.clone(), want),
            _ => self.adapt(from, want),
        }
    }

    /// Brings a value of the value class `c` applied to `args`, left by a node as `t`, to
    /// `want`: a boxed one is unboxed, and the underlying value is boxed where a reference
    /// other than the class's erasure is wanted (a type parameter, `Any`, a trait the class
    /// extends).
    pub(super) fn adapt_vc(&mut self, c: ClassId, args: TList, t: JType, want: &JType) {
        self.adapt_vc_as(c, args, t, want, true)
    }

    /// `adapt_vc`, where a reference that is not statically the box is unboxed after a null test
    /// (`null_safe`), a null giving the underlying type's zero, as dotty's `Boxing.unbox` does for
    /// a tree that is no subtype of the class (`Erasure.scala` 292-317); a box that is
    /// statically one, and a selection of the class's parameter, go through the accessor.
    fn adapt_vc_as(&mut self, c: ClassId, args: TList, t: JType, want: &JType, null_safe: bool) {
        let u = self.vc_erasure(c, args);
        // A reference wanted that the value class is not (a jar descriptor's `Encoder` for
        // circe's `Exported[ConfiguredEncoder[Mode]]`, scalac's erasure of the type the
        // parameter declares) is the underlying value's own.
        let underlying_wanted = u.is_ref() && matches!(want, JType::L(n) if !n.starts_with('[') && !self.is_vc_mark(want) && !want.is_object() && !self.vc_conforms(c, n));
        let boxing = want.is_ref() && !self.holds_underlying(want, &u) && !underlying_wanted;
        // A reference that holds the box goes to a destination that takes the box as it is, a
        // null and the reference's identity kept: dotty keeps a tree that conforms (`Erasure.scala`
        // 392-393) and unboxes only toward an `ErasedValueType` (399-400), where the null branch is
        // (292-317).
        if boxing && !self.holds_underlying(&t, &u) {
            return self.adapt(&t, want);
        }
        let held = if self.holds_underlying(&t, &u) {
            t
        } else {
            let boxed = self.class_type(c);
            let declared = self.vc_underlying(c);
            if null_safe && t != boxed {
                // The value is evaluated once: tested on a copy, unboxed or replaced by the zero.
                let unbox = self.code.new_label();
                let done = self.code.new_label();
                self.dup();
                self.jump_if(op::IFNONNULL, 1, unbox);
                self.pop_value(&t);
                self.vc_zero(&declared);
                self.goto(done);
                self.code.bind(unbox);
                self.adapt(&t, &boxed);
                self.vc_unbox(c, &declared);
                self.code.bind(done);
            } else {
                self.adapt(&t, &boxed);
                self.vc_unbox(c, &declared);
            }
            declared
        };
        if boxing {
            self.adapt(&held, &u);
            self.vc_box(c, &u);
            let boxed = self.class_type(c);
            self.adapt(&boxed, want);
        } else {
            self.adapt(&held, want);
        }
    }

    /// The zero of a value class's underlying type, what a null unboxes to; a `null` is of the
    /// underlying type in the frame of the join it flows to.
    fn vc_zero(&mut self, u: &JType) {
        match u {
            JType::J => self.lconst(0),
            JType::D => self.dconst(0.0),
            JType::F => self.fconst(0.0),
            JType::L(_) => {
                self.code.op(op::ACONST_NULL);
                let vt = self.vt(u);
                self.code.push(vt);
            }
            _ => self.iconst(0),
        }
    }

    /// Whether the value class `c` itself is a `name`: its own class or one of its parents.
    fn vc_conforms(&mut self, c: ClassId, name: &str) -> bool {
        if &*self.class_name(c) == name {
            return true;
        }
        let bases: Vec<ClassId> = self.cx.input.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        bases.into_iter().any(|b| &*self.class_name(b) == name)
    }

    /// The type of a local, a field or a parameter: `Unit` is a reference there.
    pub fn storage(&mut self, t: TypeId) -> JType {
        match self.erase(t) {
            JType::V => JType::L(Rc::from(BOXED_UNIT)),
            j => j,
        }
    }

    /// The type a symbol's value has where it is stored.
    pub fn sym_type(&mut self, s: SymId) -> JType {
        // A given object of an instance is read as scalac's inner module class.
        if let Some(&k) = self.cx.inner_given_objects.get(&s) {
            return self.class_type(k);
        }
        let info = self.cx.input.syms.sym(s);
        if info.by_name {
            return self.function_type(0);
        }
        match &info.sig {
            Some(sig) if self.type_written(s) => self.written_bottom(sig.ret).unwrap_or_else(|| self.storage(sig.ret)),
            Some(sig) => self.storage(sig.ret),
            None => JType::object(),
        }
    }

    pub fn vt(&mut self, t: &JType) -> VT {
        match t {
            JType::V => VT::Top,
            JType::Z | JType::B | JType::S | JType::C | JType::I => VT::Int,
            JType::J => VT::Long,
            JType::F => VT::Float,
            JType::D => VT::Double,
            JType::L(n) => VT::Object(self.cw.cp.class(n)),
        }
    }

    // ---- names and references ----

    pub fn method_name(&mut self, s: SymId) -> String {
        if let Some(target) = self.target_name(s) {
            return target;
        }
        self.source_method_name(s)
    }

    /// The name of method `s` in the class files as its source name gives it, `@targetName`
    /// apart: what the names of its default getters begin with (`$plus$default$1` of
    /// `@targetName("renamed") def +(x: Int = 1)`), as scalac names them.
    pub fn source_method_name(&mut self, s: SymId) -> String {
        let syms = self.cx.input.syms;
        let info = syms.sym(s);
        // A jar's member called already: the name its class file gives it, its `@targetName`
        // aside.
        if self.linked_owner(s).is_some() && self.target_name(s).is_none() {
            if let Some(m) = self.mrefs.get(&s) {
                return m.name.clone();
            }
        }
        let mut name = encode(self.cx.input.interner.get(info.name));
        if info.is_extension {
            let siblings: &[SymId] = match info.owner {
                Owner::Class(c) => &syms.class(c).extensions,
                Owner::Package(p) => syms.pkg(p).entries.get(&info.name).map_or(&[], |e| &e.extensions),
                Owner::Local => &[],
            };
            let mut same: Vec<SymId> = siblings.iter().copied().filter(|&e| syms.sym(e).name == info.name).collect();
            same.sort_by(|&a, &b| crate::emit::layout::compare_syms(syms, self.cx.input.interner, a, b));
            let member_too = matches!(info.owner, Owner::Class(c) if syms.class(c).members.contains_key(&info.name));
            if let Some(rank) = same.iter().position(|&e| e == s) {
                if rank > 0 || member_too || (same.len() > 1 && matches!(info.owner, Owner::Package(_))) {
                    name.push_str(&format!("$x{}", rank + member_too as usize));
                }
            }
        }
        self.expanded_private_name(s, name)
    }

    /// scalac's expanded name `Class$$name` for a private member whose name a class above or
    /// below uses as well.
    fn expanded_private_name(&self, s: SymId, name: String) -> String {
        let syms = self.cx.input.syms;
        let info = syms.sym(s);
        match info.owner {
            Owner::Class(c)
                if info.mods & mods::PRIVATE != 0
                    && !info.scoped_private
                    && (syms.private_name_clashes(c, info.name) || (self.cx.is_interface(c) && matches!(info.kind, SymKind::Val | SymKind::Var))) =>
            {
                format!("{}$${}", self.cx.class_names[c.idx()].replace('/', "$"), name)
            }
            _ => name,
        }
    }

    /// `@targetName("name")`: the name of the method in the class file, as under scalac, its
    /// bridges' and forwarders' too.
    pub fn target_name(&self, s: SymId) -> Option<String> {
        if let Some(&n) = self.cx.input.target_names.and_then(|m| m.get(&s)).or_else(|| self.cx.input.product_target_names.and_then(|m| m.get(&s))) {
            return Some(encode(self.cx.input.interner.get(n)));
        }
        let &n = self.cx.input.source_target_names?.get(&s)?;
        Some(self.cx.input.interner.get(n).to_string())
    }

    /// The flattened parameters of a def: by-name ones are functions, repeated ones sequences.
    pub fn param_types(&mut self, s: SymId) -> Vec<JType> {
        let cx = self.cx;
        if let Some((_, JMember::Method(_))) = cx.input.java_member(s) {
            return self.mref(s).params.clone();
        }
        if self.linked_owner(s).is_some() {
            return self.mref(s).params.clone();
        }
        self.erased_param_types(s)
    }

    /// A parameter's type as the signature spells it where that is a `*:` chain, which the
    /// body reads as the tuple class of its elements (`ParamSig::ty`). In the order the class
    /// file takes them (`jvm_param_order`).
    fn erased_param_types(&mut self, s: SymId) -> Vec<JType> {
        let cx = self.cx;
        let Some(sig) = cx.input.syms.sym(s).info.sig.as_ref() else { return Vec::new() };
        let written = cx.input.syms.sym(s).def.is_some();
        let params: Vec<(SymId, Option<TypeId>)> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| (p.sym, (!p.by_name && !p.repeated).then_some(p.ty)))).collect();
        let types: Vec<JType> = params
            .into_iter()
            .map(|(p, declared)| match declared {
                Some(t) if self.is_cons_chain(t) => self.storage(t),
                Some(t) if written && self.written_bottom(t).is_some() => self.written_bottom(t).unwrap_or_else(JType::object),
                _ => self.sym_type(p),
            })
            .collect();
        match self.jvm_param_order(s) {
            Some(order) => order.iter().map(|&k| types[k].clone()).collect(),
            None => types,
        }
    }

    /// The parameters of a right-associative extension method in the order its class file takes
    /// them, as scalac's `paramss` hold them: the method's first explicit clause before the
    /// receiver's (`$plus$colon(String, int)` of `extension (x: Int) def +:(s: String)`), each
    /// by its index in the signature's flattened order, which puts the receiver first. None for
    /// every other method, whose order is the signature's.
    pub fn jvm_param_order(&self, s: SymId) -> Option<Vec<usize>> {
        let cx = self.cx;
        let clauses = self.declared_clauses(s)?;
        let sig = cx.input.syms.sym(s).info.sig.as_ref()?;
        let starts: Vec<usize> = sig.clauses.iter().scan(0, |acc, c| {
            let at = *acc;
            *acc += c.params.len();
            Some(at)
        }).collect();
        let order: Vec<usize> = clauses.iter().flat_map(|&c| starts[c]..starts[c] + sig.clauses[c].params.len()).collect();
        (order.iter().enumerate().any(|(j, &k)| j != k)).then_some(order)
    }

    /// The clauses of a right-associative extension method as scalac's `paramss` hold them, by
    /// their indices in the signature: the method's first explicit clause before the receiver's.
    /// None for every other method.
    fn declared_clauses(&self, s: SymId) -> Option<Vec<usize>> {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        if !info.is_extension || !cx.input.interner.get(info.name).ends_with(':') {
            return None;
        }
        info.sig.as_ref()?.right_assoc_order(info.ext_clauses as usize)
    }

    /// Whether `t` is a `*:` chain or an array of one.
    fn is_cons_chain(&self, t: TypeId) -> bool {
        let types = self.cx.input.types;
        match types.get(t) {
            Type::Class(c, _) if scala_class(self.cx, c) == Some("*:") => true,
            Type::Class(c, args) if self.cx.input.syms.class(c).kind == ClassKind::Builtin && self.cx.input.interner.get(self.cx.input.syms.class(c).name) == "Array" => {
                types.items(args).first().map_or(false, |&e| self.is_cons_chain(e))
            }
            _ => false,
        }
    }

    pub fn return_type(&mut self, s: SymId) -> JType {
        let cx = self.cx;
        if let Some((_, JMember::Method(_))) = cx.input.java_member(s) {
            let ret = self.mref(s).ret.clone();
            return self.java_value_type(ret);
        }
        if self.linked_owner(s).is_some() {
            return self.mref(s).ret.clone();
        }
        self.erased_return_type(s)
    }

    fn erased_return_type(&mut self, s: SymId) -> JType {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        let Some(sig) = info.sig.as_ref() else { return JType::object() };
        let ret = sig.ret;
        // `x.type` of a parameter the signature spells as a `*:` chain is the chain's.
        let spelled = match cx.input.types.get(ret) {
            Type::Term(x) => sig.clauses.iter().flat_map(|c| c.params.iter()).find(|p| p.sym == x && !p.by_name && !p.repeated).map(|p| p.ty),
            _ => None,
        };
        if let Some(t) = spelled.filter(|&t| self.is_cons_chain(t)) {
            return self.storage(t);
        }
        match self.type_written(s) {
            true => self.written_bottom(ret).unwrap_or_else(|| self.erase(ret)),
            false => self.erase(ret),
        }
    }

    pub fn mref(&mut self, s: SymId) -> Rc<MRef> {
        if let Some(m) = self.mrefs.get(&s) {
            let m = m.clone();
            if let Some(&used) = self.mref_arities.get(&s) {
                self.function_arities |= used;
            }
            return m;
        }
        let before = std::mem::take(&mut self.function_arities);
        let m = self.make_mref(s);
        self.keep_arities(before, |g, used| g.mref_arities.insert(s, used));
        m
    }

    fn make_mref(&mut self, s: SymId) -> Rc<MRef> {
        let cx = self.cx;
        // A Java member is called as its class file declares it: the descriptor is exact.
        if let Some((cf, JMember::Method(i))) = cx.input.java_member(s) {
            let jm = &cf.methods[i as usize];
            let (params, ret) = parse_method_desc(&jm.descriptor);
            let kind = if jm.access & ACC_STATIC != 0 {
                Invoke::Static
            } else if cf.is_interface() {
                Invoke::Interface
            } else {
                Invoke::Virtual
            };
            let m = Rc::new(MRef {
                owner: Rc::from(cf.name.as_str()),
                owner_is_interface: cf.is_interface(),
                name: jm.name.clone(),
                params,
                ret,
                desc: jm.descriptor.clone(),
                kind,
            });
            self.mrefs.insert(s, m.clone());
            return m;
        }
        let info = cx.input.syms.sym(s);
        let params = self.erased_param_types(s);
        let is_value = !matches!(info.kind, SymKind::Def) && !(info.kind == SymKind::Given && !params.is_empty());
        let ret = if is_value { self.sym_type(s) } else { self.erased_return_type(s) };
        if let Some(m) = self.linked_mref(s, is_value, &params, &ret) {
            let m = Rc::new(m);
            self.mrefs.insert(s, m.clone());
            return m;
        }
        // `@jvmWide`: a def of the standard library whose result is not always what its type
        // says (`Map.map` reached through `Iterable` gives a `List` for a function without pairs).
        let wide = info.def.map_or(false, |d| cx.input.asts[info.file.0 as usize].def(d).annots.iter().any(|a| a.name == crate::names::JVM_WIDE));
        let ret = if wide && ret.is_ref() { JType::object() } else { ret };
        let (owner, owner_is_interface, kind): (Rc<str>, bool, Invoke) = match info.owner {
            Owner::Class(c) if cx.is_interface(c) => (self.class_name(c), true, Invoke::Interface),
            // An interface's static is an `InterfaceMethodref` (`Function.identity`).
            Owner::Class(c) if cx.jdk_statics(c) => {
                let interface = cx.input.syms.class(c).companion.map_or(false, |t| cx.is_interface(t));
                (self.class_name(c), interface, Invoke::Static)
            }
            Owner::Class(c) => (self.class_name(c), false, Invoke::Virtual),
            _ => (Rc::from(cx.file_modules[info.file.0 as usize].as_str()), false, Invoke::Virtual),
        };
        let name = if is_value { self.field_name(s) } else { self.method_name(s) };
        // A given class's def answers its class, as scalac's (`Cx::given_classes`).
        let ret = match cx.given_classes.get(&s) {
            Some(&k) => self.class_type(k),
            None => ret,
        };
        let desc = method_desc(&params, &ret);
        let m = Rc::new(MRef { owner, owner_is_interface, name, params, ret, desc, kind });
        self.mrefs.insert(s, m.clone());
        m
    }

    /// In link mode, the class file of the jar class that declares `s`.
    fn linked_owner(&self, s: SymId) -> Option<&'a crate::classfile::ClassFile> {
        let cx: &'a Cx<'a> = self.cx;
        match cx.input.syms.sym(s).owner {
            Owner::Class(c) => cx.class_files.get(&c).map(|cf| &**cf),
            _ => None,
        }
    }

    /// A member of a jar class is called as its class file declares it, as a Java member is:
    /// the method of its name and arity whose parameters match teq's erasure best. Where the
    /// descriptor's types are not the erasure's, the call's arguments and result adapt to them.
    fn linked_mref(&mut self, s: SymId, is_value: bool, params: &[JType], ret: &JType) -> Option<MRef> {
        let cf = self.linked_owner(s)?;
        let cx = self.cx;
        let name = self.target_name(s).unwrap_or_else(|| encode(cx.input.interner.get(cx.input.syms.sym(s).name)).to_string());
        let arity = if is_value { 0 } else { params.len() };
        let own = method_desc(if is_value { &[] } else { params }, ret);
        // scalac mangles an accessor of an expanded name with the prefix inside the expansion:
        // TASTy's `inline$p$$x` is the class file's `p$$inline$x` (pureconfig's accessor of a
        // trait parameter).
        let mut name = name;
        let mut picked = pick_method(cf, &name, arity, &own);
        // teq's class file names an extension's overloads apart, `m`, `m$x1`, ..
        // (`method_name`): the one whose parameters are the call's.
        if cx.input.syms.sym(s).is_extension && picked.map_or(true, |m| m.descriptor != own) {
            let suffixed = cf.methods.iter().find(|m| m.descriptor == own && m.name.strip_prefix(name.as_str()).and_then(|r| r.strip_prefix("$x")).map_or(false, |k| !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit())));
            if let Some(m) = suffixed {
                name = m.name.clone();
                picked = Some(m);
            }
        }
        if picked.is_none() {
            if let Some(mangled) = name.strip_prefix("inline$").and_then(|r| r.rsplit_once("$$")).map(|(p, x)| format!("{}$$inline${}", p, x)) {
                picked = pick_method(cf, &mangled, arity, &own);
                if picked.is_some() {
                    name = mangled;
                }
            }
        }
        // A private member an inline body reaches goes through the `inline$m` accessor scalac
        // made for it, the class file's public method (magnolia's `returningNone`).
        let (m, name) = match picked {
            Some(m) if m.access & ACC_PRIVATE == 0 => (m, name),
            _ => {
                let accessor = format!("inline${}", name);
                match pick_method(cf, &accessor, arity, &own) {
                    Some(a) => (a, accessor),
                    None => (picked?, name),
                }
            }
        };
        let (desc_params, desc_ret) = parse_method_desc(&m.descriptor);
        // An `Object` the erasure marks as a value class's underlying value keeps the mark, also
        // where the descriptor names the instantiated underlying type's erasure (`Encoder` for
        // kantan's `Exported[CellEncoder[LocalDate]]`): a jar passes the value class as its
        // underlying value wherever its signature names the class.
        let retag = |d: JType, o: Option<&JType>| match o {
            Some(o) if *o == d || (self.is_vc_object(o) && d.is_ref()) => o.clone(),
            _ => d,
        };
        let params = desc_params.into_iter().enumerate().map(|(i, d)| retag(d, params.get(i))).collect();
        let ret = retag(desc_ret, Some(ret));
        Some(MRef {
            owner: Rc::from(cf.name.as_str()),
            owner_is_interface: cf.is_interface(),
            name,
            params,
            ret,
            desc: m.descriptor.clone(),
            kind: if cf.is_interface() { Invoke::Interface } else { Invoke::Virtual },
        })
    }

    /// The name of the field that holds a member and of its accessor. A trait's parameter is
    /// scalac's: a `val` its own name, a private one expanded (`T$$x`, as `Mixin` makes a
    /// trait's private members public); an anonymous using parameter's is the `x$N` the parser
    /// names it, scalac's.
    pub fn field_name(&mut self, s: SymId) -> String {
        let cx = self.cx;
        let base = encode(cx.input.interner.get(cx.input.syms.sym(s).name));
        self.expanded_private_name(s, base)
    }

    // ---- instructions that keep the model in step ----

    pub fn iconst(&mut self, v: i32) {
        match v {
            -1..=5 => self.code.op((op::ICONST_0 as i32 + v) as u8),
            -128..=127 => self.code.op_u8(op::BIPUSH, v as u8),
            -32768..=32767 => self.code.op_u16(op::SIPUSH, v as u16),
            _ => {
                let i = self.cw.cp.int(v);
                self.ldc(i);
            }
        }
        self.code.push(VT::Int);
    }

    fn ldc(&mut self, index: u16) {
        if index <= 255 {
            self.code.op_u8(op::LDC, index as u8);
        } else {
            self.code.op_u16(op::LDC_W, index);
        }
    }

    pub fn lconst(&mut self, v: i64) {
        if v == 0 || v == 1 {
            self.code.op(op::LCONST_0 + v as u8);
        } else {
            let i = self.cw.cp.long(v);
            self.code.op_u16(op::LDC2_W, i);
        }
        self.code.push(VT::Long);
    }

    pub fn dconst(&mut self, v: f64) {
        if v.to_bits() == 0f64.to_bits() || v == 1.0 {
            self.code.op(op::DCONST_0 + v as u8);
        } else {
            let i = self.cw.cp.double(v);
            self.code.op_u16(op::LDC2_W, i);
        }
        self.code.push(VT::Double);
    }

    pub fn fconst(&mut self, v: f32) {
        if v.to_bits() == 0f32.to_bits() || v == 1.0 || v == 2.0 {
            self.code.op(op::FCONST_0 + v as u8);
        } else {
            let i = self.cw.cp.float(v);
            self.ldc(i);
        }
        self.code.push(VT::Float);
    }

    pub fn sconst(&mut self, s: &str) {
        let i = self.cw.cp.string(s);
        self.ldc(i);
        let t = VT::Object(self.cw.cp.class(STRING));
        self.code.push(t);
    }

    pub fn load(&mut self, slot: u16, t: &JType) {
        let (base, short) = match t {
            JType::Z | JType::B | JType::S | JType::C | JType::I => (op::ILOAD, op::ILOAD_0),
            JType::J => (op::LLOAD, op::LLOAD_0),
            JType::F => (op::FLOAD, op::FLOAD_0),
            JType::D => (op::DLOAD, op::DLOAD_0),
            JType::L(_) => (op::ALOAD, op::ALOAD_0),
            JType::V => return,
        };
        self.code.var_insn(base, short, slot);
        let vt = match self.code.locals.get(slot as usize) {
            Some(&v @ (VT::UninitThis | VT::Uninit(_))) => v,
            _ => self.vt(t),
        };
        self.code.push(vt);
    }

    pub fn store(&mut self, slot: u16, t: &JType) {
        let (base, short) = match t {
            JType::Z | JType::B | JType::S | JType::C | JType::I => (op::ISTORE, op::ISTORE_0),
            JType::J => (op::LSTORE, op::LSTORE_0),
            JType::F => (op::FSTORE, op::FSTORE_0),
            JType::D => (op::DSTORE, op::DSTORE_0),
            JType::L(_) => (op::ASTORE, op::ASTORE_0),
            JType::V => return,
        };
        self.code.var_insn(base, short, slot);
        self.code.pop();
    }

    /// A fresh slot holding the value on the stack.
    pub fn store_new(&mut self, t: &JType) -> u16 {
        let vt = self.vt(t);
        let slot = self.code.new_local(vt);
        self.store(slot, t);
        slot
    }

    pub fn pop_value(&mut self, t: &JType) {
        match t {
            JType::V => {}
            JType::J | JType::D => {
                self.code.op(op::POP2);
                self.code.pop();
            }
            _ => {
                self.code.op(op::POP);
                self.code.pop();
            }
        }
    }

    pub fn dup(&mut self) {
        let top = *self.code.stack.last().expect("dup of an empty stack");
        self.code.op(if matches!(top, VT::Long | VT::Double) { op::DUP2 } else { op::DUP });
        self.code.push(top);
    }

    pub fn invoke(&mut self, kind: Invoke, owner: &str, owner_is_interface: bool, name: &str, params: &[JType], ret: &JType) {
        let desc = method_desc(params, ret);
        self.invoke_desc(kind, owner, owner_is_interface, name, &desc, params.len(), ret);
    }

    pub fn invoke_desc(&mut self, kind: Invoke, owner: &str, owner_is_interface: bool, name: &str, desc: &str, n_params: usize, ret: &JType) {
        let index = if owner_is_interface {
            self.cw.cp.interface_method(owner, name, desc)
        } else {
            self.cw.cp.method(owner, name, desc)
        };
        match kind {
            Invoke::Virtual => self.code.op_u16(op::INVOKEVIRTUAL, index),
            Invoke::Static => self.code.op_u16(op::INVOKESTATIC, index),
            Invoke::Special => self.code.op_u16(op::INVOKESPECIAL, index),
            Invoke::Interface => {
                self.code.op_u16(op::INVOKEINTERFACE, index);
                let words: usize = 1 + parse_method_desc(desc).0.iter().map(|p| if p.wide() { 2 } else { 1 }).sum::<usize>();
                self.code.bytes.push(words as u8);
                self.code.bytes.push(0);
            }
        }
        self.code.popn(n_params);
        if kind != Invoke::Static {
            let receiver = self.code.pop();
            if name == "<init>" {
                let initialised = VT::Object(self.cw.cp.class(owner));
                let this_type = VT::Object(self.cw.cp.class(&self.this_name.clone()));
                for t in self.code.stack.iter_mut().chain(self.code.locals.iter_mut()) {
                    if *t == receiver {
                        *t = if receiver == VT::UninitThis { this_type } else { initialised };
                    }
                }
            }
        }
        if *ret != JType::V {
            let vt = self.vt(ret);
            self.code.push(vt);
        }
    }

    pub fn invoke_mref(&mut self, m: &MRef) {
        self.invoke_desc(m.kind, &m.owner, m.owner_is_interface, &m.name, &m.desc, m.params.len(), &m.ret);
    }

    pub fn new_object(&mut self, class: &str) {
        let offset = self.code.pc() as u16;
        let index = self.cw.cp.class(class);
        self.code.op_u16(op::NEW, index);
        self.code.push(VT::Uninit(offset));
        self.dup();
    }

    pub fn getstatic(&mut self, owner: &str, name: &str, t: &JType) {
        let mut d = String::new();
        t.desc(&mut d);
        let f = self.cw.cp.field(owner, name, &d);
        self.code.op_u16(op::GETSTATIC, f);
        let vt = self.vt(t);
        self.code.push(vt);
    }

    pub fn putstatic(&mut self, owner: &str, name: &str, t: &JType) {
        let mut d = String::new();
        t.desc(&mut d);
        let f = self.cw.cp.field(owner, name, &d);
        self.code.op_u16(op::PUTSTATIC, f);
        self.code.pop();
    }

    pub fn getfield(&mut self, owner: &str, name: &str, t: &JType) {
        let mut d = String::new();
        t.desc(&mut d);
        let f = self.cw.cp.field(owner, name, &d);
        self.code.op_u16(op::GETFIELD, f);
        self.code.pop();
        let vt = self.vt(t);
        self.code.push(vt);
    }

    pub fn putfield(&mut self, owner: &str, name: &str, t: &JType) {
        let mut d = String::new();
        t.desc(&mut d);
        let f = self.cw.cp.field(owner, name, &d);
        self.code.op_u16(op::PUTFIELD, f);
        self.code.popn(2);
    }

    pub fn checkcast(&mut self, class: &str) {
        let index = self.cw.cp.class(class);
        self.code.op_u16(op::CHECKCAST, index);
        self.code.pop();
        self.code.push(VT::Object(index));
    }

    pub fn instance_of(&mut self, class: &str) {
        let index = self.cw.cp.class(class);
        self.code.op_u16(op::INSTANCEOF, index);
        self.code.pop();
        self.code.push(VT::Int);
    }

    /// A conditional jump that pops `pops` operands.
    pub fn jump_if(&mut self, opcode: u8, pops: usize, target: Label) {
        self.code.popn(pops);
        self.code.jump(opcode, target);
    }

    pub fn goto(&mut self, target: Label) {
        self.code.jump(op::GOTO, target);
    }

    pub fn return_value(&mut self, t: &JType) {
        let opcode = match t {
            JType::V => op::RETURN,
            JType::Z | JType::B | JType::S | JType::C | JType::I => op::IRETURN,
            JType::J => op::LRETURN,
            JType::F => op::FRETURN,
            JType::D => op::DRETURN,
            JType::L(_) => op::ARETURN,
        };
        self.code.op(opcode);
        if *t != JType::V {
            self.code.pop();
        }
        self.code.end_path();
    }

    pub fn unit_value(&mut self) {
        let t = JType::L(Rc::from(BOXED_UNIT));
        self.getstatic(BOXED_UNIT, "UNIT", &t);
    }

    // ---- adaptation ----

    /// Whether the verifier takes a `from` for a `to` without a cast: an interface is not
    /// assignable to a class, whatever superclass its trait has.
    fn is_subclass(&self, from: &str, to: &str) -> bool {
        let cx = self.cx;
        let by_name = if cx.input.open_world { &cx.all_by_name } else { &cx.class_by_name };
        let (Some(&f), Some(&t)) = (by_name.get(from), by_name.get(to)) else { return false };
        let syms = cx.input.syms;
        if syms.class(f).kind == ClassKind::Trait && syms.class(t).kind != ClassKind::Trait {
            return false;
        }
        // A given object of an instance implements its given's declared type, which a build over
        // its products, which may not see its class's parents, reads as this one does.
        if let Some(&g) = cx.inner_given_object_classes.get(&f) {
            let declared = syms.sym(g).sig.as_ref().and_then(|sig| match cx.input.types.get(sig.ret) {
                crate::types::Type::Class(d, _) => Some(d),
                _ => None,
            });
            if declared.map_or(false, |d| d == t || syms.class(d).base_types.iter().any(|&(b, _)| b == t)) {
                return true;
            }
        }
        syms.class(f).base_types.iter().any(|&(b, _)| b == t)
    }

    pub fn adapt(&mut self, from: &JType, to: &JType) {
        if from == to {
            return;
        }
        match (from, to) {
            (_, JType::V) => self.pop_value(from),
            (JType::V, JType::L(_)) => {
                self.unit_value();
                let vt = self.vt(to);
                self.code.retype_top(vt);
            }
            (JType::V, JType::J) => self.lconst(0),
            (JType::V, JType::F) => self.fconst(0.0),
            (JType::V, JType::D) => self.dconst(0.0),
            (JType::V, _) => self.iconst(0),
            (JType::L(f), JType::L(t)) => {
                if &**t == OBJECT || self.is_subclass(f, t) {
                    let vt = self.vt(to);
                    self.code.retype_top(vt);
                } else {
                    self.checkcast(t);
                }
            }
            // A Char is a String of one character where the typer takes one for the other.
            (JType::L(f), JType::C) if &**f == STRING => {
                self.iconst(0);
                self.invoke_desc(Invoke::Virtual, STRING, false, "charAt", "(I)C", 1, &JType::C);
            }
            (JType::C, JType::L(t)) if &**t == STRING => {
                self.invoke_desc(Invoke::Static, STRING, false, "valueOf", "(C)Ljava/lang/String;", 1, to);
            }
            (JType::L(_), prim) => {
                let (class, method, desc) = match prim {
                    JType::Z => ("java/lang/Boolean", "booleanValue", "()Z"),
                    JType::C => ("java/lang/Character", "charValue", "()C"),
                    JType::B => ("java/lang/Number", "byteValue", "()B"),
                    JType::S => ("java/lang/Number", "shortValue", "()S"),
                    JType::I => ("java/lang/Number", "intValue", "()I"),
                    JType::J => ("java/lang/Number", "longValue", "()J"),
                    JType::F => ("java/lang/Number", "floatValue", "()F"),
                    _ => ("java/lang/Number", "doubleValue", "()D"),
                };
                // scalac's erasure unboxes through `BoxesRunTime.unboxToX(Object)` (Erasure.scala:
                // 319-324), one static call that gives the primitive's zero for `null` (a
                // `java.util.Map[K, Int]`'s missing key is 0) and refuses another kind's box; where
                // the class path lacks scala-library the same is spelled out: the value or a boxed
                // zero by `Objects.requireNonNullElse`, then `Number.xValue()`. No branch: a frame in
                // the middle of an expression would fix the operands under the value to the model's
                // wider types.
                if self.cx.boxes_runtime {
                    let (m, d) = match prim {
                        JType::Z => ("unboxToBoolean", "(Ljava/lang/Object;)Z"),
                        JType::C => ("unboxToChar", "(Ljava/lang/Object;)C"),
                        JType::B => ("unboxToByte", "(Ljava/lang/Object;)B"),
                        JType::S => ("unboxToShort", "(Ljava/lang/Object;)S"),
                        JType::I => ("unboxToInt", "(Ljava/lang/Object;)I"),
                        JType::J => ("unboxToLong", "(Ljava/lang/Object;)J"),
                        JType::F => ("unboxToFloat", "(Ljava/lang/Object;)F"),
                        _ => ("unboxToDouble", "(Ljava/lang/Object;)D"),
                    };
                    self.invoke_desc(Invoke::Static, "scala/runtime/BoxesRunTime", false, m, d, 1, prim);
                    return;
                }
                let object = JType::object();
                match prim {
                    JType::Z => self.getstatic("java/lang/Boolean", "FALSE", &JType::L(Rc::from("java/lang/Boolean"))),
                    JType::C => {
                        self.iconst(0);
                        self.invoke_desc(Invoke::Static, "java/lang/Character", false, "valueOf", "(C)Ljava/lang/Character;", 1, &JType::L(Rc::from("java/lang/Character")));
                    }
                    _ => {
                        self.iconst(0);
                        self.invoke_desc(Invoke::Static, "java/lang/Integer", false, "valueOf", "(I)Ljava/lang/Integer;", 1, &JType::L(Rc::from("java/lang/Integer")));
                    }
                }
                self.invoke_desc(Invoke::Static, "java/util/Objects", false, "requireNonNullElse", "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;", 2, &object);
                self.checkcast(class);
                self.invoke_desc(Invoke::Virtual, class, false, method, desc, 0, prim);
            }
            (prim, JType::L(t)) => {
                let class = prim.box_class();
                let mut desc = String::from("(");
                prim.desc(&mut desc);
                desc.push_str(")L");
                desc.push_str(class);
                desc.push(';');
                let boxed = JType::L(Rc::from(class));
                self.invoke_desc(Invoke::Static, class, false, "valueOf", &desc, 1, &boxed);
                if &**t != class {
                    let vt = self.vt(to);
                    self.code.retype_top(vt);
                }
            }
            (a, b) => {
                // Between the stack classes first (an int, a long, a float, a double), then
                // narrowed to the small integer type wanted.
                let mut insns: Vec<u8> = Vec::with_capacity(2);
                match (a.stack_type(), b.stack_type()) {
                    (JType::I, JType::J) => insns.push(op::I2L),
                    (JType::I, JType::F) => insns.push(op::I2F),
                    (JType::I, JType::D) => insns.push(op::I2D),
                    (JType::J, JType::I) => insns.push(op::L2I),
                    (JType::J, JType::F) => insns.push(op::L2F),
                    (JType::J, JType::D) => insns.push(op::L2D),
                    (JType::F, JType::I) => insns.push(op::F2I),
                    (JType::F, JType::J) => insns.push(op::F2L),
                    (JType::F, JType::D) => insns.push(op::F2D),
                    (JType::D, JType::I) => insns.push(op::D2I),
                    (JType::D, JType::J) => insns.push(op::D2L),
                    (JType::D, JType::F) => insns.push(op::D2F),
                    _ => {}
                }
                let narrowed_already = matches!((a, b), (JType::B, JType::S) | (JType::B | JType::S | JType::Z, JType::I));
                if !narrowed_already {
                    match b {
                        JType::B => insns.push(op::I2B),
                        JType::S => insns.push(op::I2S),
                        JType::C => insns.push(op::I2C),
                        _ => {}
                    }
                }
                if !insns.is_empty() {
                    for i in insns {
                        self.code.op(i);
                    }
                    self.code.pop();
                    let vt = self.vt(b);
                    self.code.push(vt);
                }
            }
        }
    }

    // ---- static types ----

    pub(super) fn recorded_type(&self, e: TExprId) -> Option<TypeId> {
        self.cx.input.prog.expr_types.get(e.0).filter(|&t| t != NO_TYPE)
    }

    pub fn recorded_erasure(&mut self, e: TExprId) -> JType {
        self.recorded(e).unwrap_or_else(JType::object)
    }

    fn recorded(&mut self, e: TExprId) -> Option<JType> {
        let t = self.cx.input.prog.expr_types.get(e.0)?;
        if t == NO_TYPE {
            return None;
        }
        Some(self.erase(t))
    }

    fn join(&self, a: JType, b: JType) -> JType {
        if a == b {
            return a;
        }
        match (a.numeric_rank(), b.numeric_rank()) {
            (Some(x), Some(y)) => {
                if x >= y {
                    a
                } else {
                    b
                }
            }
            _ if a == JType::V || b == JType::V => JType::V,
            _ => JType::object(),
        }
    }

    /// The numeric type the typer gave a literal where that is narrower than its node (a `1`
    /// that is a `Byte`, a `1.5f` that is a `Float`).
    fn literal_type(&mut self, e: TExprId) -> Option<JType> {
        self.recorded(e).filter(|t| matches!(t, JType::B | JType::S | JType::F))
    }

    /// `shape`, the type a node's code leaves, or the wider numeric type the typer recorded on
    /// it: a widening between two kinds of box has a node of its own (`xs.head.toDouble` is
    /// `IntToDouble` over the selection, whose Int is unboxed as one), a literal typed narrower
    /// than its node (`val s: Short = 2`) is read through `literal_type`.
    fn widened(&mut self, e: TExprId, shape: JType) -> JType {
        match self.recorded(e) {
            Some(t) if shape.numeric_rank().is_some() && t.numeric_rank() > shape.numeric_rank() => t,
            _ => shape,
        }
    }

    /// The numeric type the typer recorded on a conditional, a match, a block or a `try` where
    /// it is wider than what its branches leave (`(if c then 1 else 2).toDouble`): a consumer
    /// that takes a reference boxes the widened value, not each branch's own.
    fn widened_branches(&mut self, e: TExprId) -> Option<JType> {
        let prog = self.cx.input.prog;
        if !matches!(prog.expr(e), TExpr::If(_, _, Some(_)) | TExpr::Match(..) | TExpr::Block(..) | TExpr::Try(_)) {
            return None;
        }
        let recorded = self.recorded(e)?;
        recorded.numeric_rank()?;
        let results: Vec<TExprId> = match prog.expr(e) {
            TExpr::If(_, t, Some(els)) => vec![t, els],
            TExpr::Match(_, cases) => prog.cases[cases.range()].iter().map(|c| c.body).collect(),
            TExpr::Block(_, res) => vec![res],
            TExpr::Try(i) => {
                let t = &prog.tries[i as usize];
                std::iter::once(t.body).chain(prog.cases[t.cases.range()].iter().map(|c| c.body)).collect()
            }
            _ => return None,
        };
        // A branch that throws leaves nothing to join.
        let mut branches: Option<JType> = None;
        for r in results {
            if matches!(prog.expr(r), TExpr::Throw(..)) || self.recorded_type(r) == Some(NOTHING) {
                continue;
            }
            let t = self.static_type(r);
            branches = Some(match branches {
                Some(b) => self.join(b, t),
                None => t,
            });
        }
        let branches = branches?;
        (branches.numeric_rank().is_some() && recorded.numeric_rank() > branches.numeric_rank()).then_some(recorded)
    }

    /// The type the expression has for a consumer without a wish of its own.
    pub fn static_type(&mut self, e: TExprId) -> JType {
        let prog = self.cx.input.prog;
        match prog.expr(e) {
            TExpr::Int(_) => match self.literal_type(e) {
                Some(t) => t,
                None => self.widened(e, JType::I),
            },
            TExpr::Long(_) => JType::J,
            TExpr::Double(_) => self.literal_type(e).unwrap_or(JType::D),
            TExpr::Bool(_) | TExpr::TypeTest(..) => JType::Z,
            TExpr::Char(_) => JType::C,
            TExpr::Str(_) | TExpr::StrConcat(_) | TExpr::ToStr(..) => JType::L(Rc::from(STRING)),
            TExpr::Unit | TExpr::While(..) | TExpr::Assign(..) | TExpr::If(_, _, None) | TExpr::Return(_) => JType::V,
            TExpr::Prim(op, ..) => self.widened(e, prim_type(op)),
            TExpr::Unary(op, _) => self.widened(e, unary_type(op)),
            TExpr::Block(_, res) => match self.recorded(e) {
                Some(t) => t,
                None => self.static_type(res),
            },
            TExpr::If(_, t, Some(els)) => match self.recorded(e) {
                Some(t) => t,
                None => {
                    let (a, b) = (self.static_type(t), self.static_type(els));
                    self.join(a, b)
                }
            },
            TExpr::Match(_, cases) => match self.recorded(e) {
                Some(t) => t,
                None => {
                    let mut out: Option<JType> = None;
                    for c in &prog.cases[cases.range()] {
                        let t = self.static_type(c.body);
                        out = Some(match out {
                            Some(o) => self.join(o, t),
                            None => t,
                        });
                    }
                    out.unwrap_or(JType::V)
                }
            },
            _ => match self.recorded(e) {
                Some(t) => t,
                None => self.natural_type(e),
            },
        }
    }

    /// What the code of a node leaves on the stack, for the nodes whose static type is not read
    /// off their shape.
    fn natural_type(&mut self, e: TExprId) -> JType {
        let prog = self.cx.input.prog;
        match prog.expr(e) {
            TExpr::Local(s) => {
                let info = self.cx.input.syms.sym(s);
                if info.by_name { self.function_type(0) } else { self.sym_type(s) }
            }
            TExpr::Static(s) | TExpr::Field(_, s) => self.sym_type(s),
            TExpr::This | TExpr::Super(_) => match self.this_class {
                Some(c) => self.class_type(c),
                None => JType::object(),
            },
            TExpr::Module(c) | TExpr::New(c, _) => self.class_type(c),
            TExpr::ClassOf(_) => JType::L(Rc::from("java/lang/Class")),
            TExpr::NewVia(s, _) => match self.cx.input.syms.sym(s).owner {
                Owner::Class(c) => self.class_type(c),
                _ => JType::object(),
            },
            TExpr::CallStatic(s, _) | TExpr::CallMethod(_, s, _) => self.return_type(s),
            TExpr::Lambda(params, _) => self.function_type(params.len as usize),
            TExpr::SeqLit(_) => match self.cx.input.array_seq {
                Some(c) => self.class_type(c),
                None => JType::object(),
            },
            TExpr::ArrayLit(_) => match self.recorded(e) {
                Some(JType::L(n)) if n.starts_with('[') => JType::L(n),
                _ => JType::L(Rc::from(OBJECT_ARRAY)),
            },
            TExpr::Spread(inner) => self.static_type(inner),
            _ => JType::object(),
        }
    }

    // ---- expressions ----

    /// Emits `e` and brings its value to the type `want`.
    pub fn expr(&mut self, e: TExprId, want: &JType) {
        let lines = (self.line_floor, self.line_hold);
        self.mark_line(e);
        self.expr_marked(e, want);
        (self.line_floor, self.line_hold) = lines;
    }

    fn expr_marked(&mut self, e: TExprId, want: &JType) {
        let prog = self.cx.input.prog;
        if want.is_ref() {
            if let Some(wider) = self.widened_branches(e) {
                self.expr(e, &wider);
                return self.adapt(&wider, want);
            }
        }
        match prog.expr(e) {
            TExpr::If(c, t, els) => self.if_(c, t, els, Mode::Value(want)),
            TExpr::Match(scrut, cases) => self.match_(scrut, cases, Mode::Value(want)),
            TExpr::Block(stmts, res) => self.block(stmts, res, Mode::Value(want)),
            TExpr::Throw(inner, _) => self.throw_(inner, want),
            TExpr::Try(i) => self.try_(i, want),
            TExpr::Int(v) if matches!(want, JType::D) => self.dconst(v as f64),
            TExpr::Int(v) if matches!(want, JType::J) => self.lconst(v as i64),
            TExpr::Int(v) if matches!(want, JType::F) => self.fconst(v as f32),
            TExpr::Double(v) if matches!(want, JType::F) => self.fconst(v as f32),
            _ => {
                let t = self.node(e);
                if let Some((c, args)) = self.expr_value_class(e) {
                    if t != JType::V {
                        if self.vc_select == Some(e) {
                            return self.adapt_vc_as(c, args, t, want, false);
                        }
                        return self.adapt_vc(c, args, t, want);
                    }
                }
                // The typer knows more than the erased signature: `xs.head` of a `List[Int]`.
                let known = match prog.expr(e) {
                    TExpr::Int(_) | TExpr::Double(_) => Some(self.static_type(e)),
                    TExpr::Long(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) => None,
                    _ => self.recorded(e),
                };
                let t = match known {
                    // Between two references the consumer's type decides; going through the
                    // static type would unbox a generic result only to box it again. A value
                    // that is dropped is not unboxed either: a Java method's null result stays.
                    Some(s) if s != t && s != JType::V && t != JType::V && *want != JType::V && !(t.is_ref() && want.is_ref() && (!s.is_ref() || want.is_object())) => {
                        self.adapt(&t, &s);
                        s
                    }
                    _ => t,
                };
                self.adapt(&t, want);
            }
        }
    }

    /// Emits `e` at its static type.
    /// Records the source line of `e` for the code that follows, as scalac does: the line of
    /// the expression's point, which for a selection is its member's name, so that a chain of
    /// calls over several lines attributes each call to its own line. An expression of another
    /// file (an inlined body) keeps the line of the call around it; an argument the typer made
    /// (a given, a default) spans the call it is in and takes the line of the call's end, where
    /// scalac places it, through its whole tree: a given's copy keeps the spans of the site that
    /// searched first, which a worker may have made at another line.
    fn mark_line(&mut self, e: TExprId) {
        let cx = self.cx;
        let prog = cx.input.prog;
        if self.line_hold {
            return;
        }
        let Some((file, span)) = prog.span_of(e) else { return };
        if file != self.line_file {
            return;
        }
        let point = match self.line_floor {
            Some((call, f, s, end)) if call != e && (f, s) == (file, span) => {
                self.line_hold = true;
                end
            }
            _ => {
                let point = match prog.expr(e) {
                    TExpr::CallMethod(recv, ..) | TExpr::Field(recv, _) => cx.point_after(file, span, prog.span_of(recv)),
                    TExpr::CallStatic(_, args) => match prog.expr_list(args).first() {
                        Some(&recv) => cx.point_after(file, span, prog.span_of(recv)),
                        None => span.start,
                    },
                    _ => span.start,
                };
                if matches!(prog.expr(e), TExpr::CallMethod(..) | TExpr::CallStatic(..) | TExpr::CallClosure(..) | TExpr::New(..) | TExpr::NewVia(..)) {
                    self.line_floor = Some((e, file, span, span.end.saturating_sub(1)));
                }
                point
            }
        };
        self.code.line(cx.line_of(file, point));
    }

    pub fn expr_static(&mut self, e: TExprId) -> JType {
        let t = self.static_type(e);
        self.expr(e, &t);
        t
    }

    pub fn statement(&mut self, e: TExprId) {
        self.expr(e, &JType::V);
    }

    /// Emits the code of one node and returns what it leaves on the stack.
    fn node(&mut self, e: TExprId) -> JType {
        let prog = self.cx.input.prog;
        match prog.expr(e) {
            TExpr::Int(v) => {
                self.iconst(v);
                // A literal the typer narrowed to the `Byte`, `Short` or `Char` expected keeps that
                // type, so that boxing it gives the narrow class (`(1: Byte): Any` is a `java.lang.Byte`).
                match self.recorded_type(e).map(|t| self.erase(t)) {
                    Some(narrow @ (JType::B | JType::S | JType::C)) => narrow,
                    _ => JType::I,
                }
            }
            TExpr::Long(v) => {
                self.lconst(v);
                JType::J
            }
            TExpr::Double(v) => {
                // A literal the typer narrowed to the `Float` expected is a float constant.
                if self.recorded_type(e).map(|t| self.erase(t)) == Some(JType::F) {
                    self.fconst(v as f32);
                    return JType::F;
                }
                self.dconst(v);
                JType::D
            }
            TExpr::Bool(v) => {
                self.iconst(v as i32);
                JType::Z
            }
            TExpr::Char(c) => {
                self.iconst(c as i32);
                JType::C
            }
            TExpr::Str(s) => {
                self.sconst(&prog.strings[s.idx()]);
                JType::L(Rc::from(STRING))
            }
            TExpr::Unit => JType::V,
            TExpr::Local(s) => self.load_local(s),
            TExpr::This => self.load_this(),
            TExpr::Super(_) => self.load_this(),
            TExpr::Static(s) => self.load_static(s),
            TExpr::Module(c) => self.load_module(c),
            TExpr::ClassOf(c) => self.class_of(e, c),
            TExpr::Field(r, s) => self.field_read(r, s),
            TExpr::CallStatic(s, args) => self.call_static(s, args),
            TExpr::CallMethod(r, s, args) => self.call_method(r, s, args),
            TExpr::CallClosure(f, args) => {
                let items = prog.expr_list(args);
                let ft = self.function_type(items.len());
                self.expr(f, &ft);
                let object = JType::object();
                for &a in items {
                    self.expr(a, &object);
                }
                let JType::L(owner) = &ft else { unreachable!() };
                let params = vec![JType::object(); items.len()];
                self.invoke(Invoke::Interface, owner, true, "apply", &params, &object);
                object
            }
            TExpr::New(c, args) => self.new_(e, c, args),
            TExpr::NewVia(s, args) => self.new_via(s, args),
            TExpr::Lambda(params, body) => self.lambda(params, body),
            TExpr::While(c, body) => {
                let head = self.code.new_label();
                let exit = self.code.new_label();
                self.code.bind(head);
                self.cond(c, exit, false);
                let mark = self.code.locals_mark();
                self.statement(body);
                self.code.locals_release(mark);
                self.goto(head);
                self.code.bind(exit);
                JType::V
            }
            TExpr::Assign(target, value) => {
                self.assign(target, value);
                JType::V
            }
            TExpr::Prim(op, a, b) => self.prim(op, a, b),
            TExpr::Unary(op, a) => self.unary(op, a),
            TExpr::StrConcat(items) => self.str_concat(items),
            TExpr::ToStr(inner, conv) => {
                // A value class's `toString` is its box's (`V@1`, a case class's `CV(1)`), as
                // dotty's `typedSelect` boxes an erased value class qualifier (`Erasure.scala`
                // 733): the program's `v.toString` is the box's method, which a null reference
                // the box stayed throws on, where a rendering writes `null`.
                if self.expr_value_class(inner).is_some() {
                    let object = JType::object();
                    let string = JType::L(Rc::from(STRING));
                    self.expr(inner, &object);
                    if conv.is_rendering() {
                        self.string_value_of(&object);
                    } else {
                        self.invoke(Invoke::Virtual, OBJECT, false, "toString", &[], &string);
                    }
                    return string;
                }
                let t = self.expr_static(inner);
                self.string_value_of(&t);
                JType::L(Rc::from(STRING))
            }
            TExpr::Js(template, args) => self.intrinsic(e, template, args),
            TExpr::TypeTest(..) | TExpr::If(..) | TExpr::Match(..) | TExpr::Block(..) => {
                let t = self.static_type(e);
                match prog.expr(e) {
                    TExpr::TypeTest(..) => self.cond_value(e),
                    _ => self.expr(e, &t),
                }
                t
            }
            TExpr::SeqLit(items) => {
                let Some(seq) = self.cx.input.array_seq else {
                    self.unsupported("varargs without ArraySeq");
                    return JType::V;
                };
                let name = self.class_name(seq);
                self.new_object(&name);
                // A JVM array of the boxed elements, which scala-library's `ArraySeq.ofRef`
                // wraps and the std's `ArraySeq` holds as the array of any kind it takes.
                let array = JType::L(Rc::from(OBJECT_ARRAY));
                let elem = self.seq_literal_element(e);
                self.boxed_array(items, &elem);
                let param = if self.cx.linked_class(seq) { array.clone() } else { JType::object() };
                self.adapt(&array, &param);
                self.invoke(Invoke::Special, &name, false, "<init>", &[param], &JType::V);
                JType::L(name)
            }
            // A JVM array of the kind its type names (an enum's `values` is a `Color[]`).
            TExpr::ArrayLit(items) => {
                let array = match self.recorded(e) {
                    Some(JType::L(n)) if n.starts_with('[') => n,
                    _ => Rc::from(OBJECT_ARRAY),
                };
                self.java_array(items, &array);
                JType::L(array)
            }
            TExpr::Index(r, i) => {
                let array = JType::L(Rc::from(OBJECT_ARRAY));
                self.expr(r, &array);
                self.iconst(i as i32);
                self.array_load(&array)
            }
            TExpr::Spread(inner) => self.expr_static(inner),
            TExpr::Return(v) => {
                if self.m.lambda {
                    self.unsupported("not supported on the JVM yet: return from a lambda");
                }
                let ret = self.m.ret.clone();
                if let Some(frame) = self.m.finalizers.last().copied() {
                    // Through the enclosing finalizer: the value kept in the frame's slot, the
                    // operands under it dropped into locals, the flag set, the try's exit taken.
                    self.expr(v, &ret);
                    if let Some(slot) = frame.value {
                        self.store(slot, &ret);
                    }
                    let _ = self.spill_stack();
                    self.iconst(1);
                    self.store(frame.pending, &JType::I);
                    self.goto(frame.exit);
                    // The code after the jump is dead but verified: it is given a `null` that the
                    // caller adapts to what it wants (a cast, an unboxing or a pop), so that the try's
                    // result slot keeps the type the live frame recorded, not a boxed unit.
                    self.code.op(op::ACONST_NULL);
                    self.code.push(VT::Null);
                    return JType::object();
                }
                self.expr(v, &ret);
                self.return_value(&ret);
                JType::V
            }
            TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::JsSelect(..) | TExpr::ObjLit(_) => {
                self.unsupported("JavaScript interop");
                JType::V
            }
            TExpr::Null => {
                self.code.op(op::ACONST_NULL);
                self.code.push(VT::Null);
                JType::object()
            }
            TExpr::Throw(..) | TExpr::Try(_) => unreachable!("emitted by expr"),
            TExpr::Splice(_) => unreachable!("a stored inline body is emitted nowhere"),
        }
    }

    /// `throw e`, followed in the model by a value of the wanted type, as the code after an
    /// `athrow` is not reached but still verified against a frame.
    fn throw_(&mut self, inner: TExprId, want: &JType) {
        let throwable = JType::L(Rc::from(THROWABLE));
        self.expr(inner, &throwable);
        self.code.op(op::ATHROW);
        self.code.pop();
        self.code.end_path();
        if *want != JType::V {
            let vt = self.vt(want);
            self.code.push(vt);
        }
    }

    /// `try`/`catch`/`finally` through the exception table. The body and the cases leave their
    /// value in a local; the finalizer is copied onto the normal exit and into a handler that
    /// takes every exception of the body and the cases and throws it again, as javac lays it
    /// out. A `try` with operands on the stack spills them to locals first, since a handler
    /// starts with an empty stack.
    fn try_(&mut self, i: u32, want: &JType) {
        let cx = self.cx;
        let prog = cx.input.prog;
        let t = &prog.tries[i as usize];
        let outer = self.code.locals_mark();
        let spilled = self.spill_stack();
        let result = if *want == JType::V {
            None
        } else {
            match want {
                JType::L(_) => {
                    self.code.op(op::ACONST_NULL);
                    self.code.push(VT::Null);
                }
                _ => self.adapt(&JType::V, want),
            }
            Some(self.store_new(want))
        };
        // A `return` inside the body or a case leaves through the finalizer, as the language runs
        // it on every exit: the value is kept in a slot, a flag set, and the normal exit taken, where
        // the finalizer runs once and the flag sends the method on (javac's lowering; scalac's prints
        // `finally ran` before `fin: 1` for `try { return 1 } finally { println("finally ran") }`).
        let pending = t.finalizer.map(|_| {
            self.iconst(0);
            self.store_new(&JType::I)
        });
        let ret = self.m.ret.clone();
        let return_slot = match (t.finalizer, &ret) {
            (None, _) | (Some(_), JType::V) => None,
            (Some(_), JType::L(_)) => {
                self.code.op(op::ACONST_NULL);
                self.code.push(VT::Null);
                Some(self.store_new(&ret))
            }
            (Some(_), _) => {
                self.adapt(&JType::V, &ret);
                Some(self.store_new(&ret))
            }
        };
        let throwable = JType::L(Rc::from(THROWABLE));
        let throwable_vt = self.vt(&throwable);
        let (start, end, normal_exit, after) =
            (self.code.new_label(), self.code.new_label(), self.code.new_label(), self.code.new_label());
        if let Some(pending) = pending {
            self.m.finalizers.push(FinallyFrame { pending, value: return_slot, exit: normal_exit });
        }
        // A handler's frame holds the locals of the whole protected range, not those a body made.
        let protected = self.code.locals_mark();
        self.code.bind(start);
        let from = self.code.pc();
        self.expr(t.body, want);
        if let Some(slot) = result {
            self.store(slot, want);
        }
        // A protected range has to hold an instruction.
        if self.code.pc() == from {
            self.code.op(op::NOP);
        }
        self.code.bind(end);
        self.goto(normal_exit);
        let mut any_ranges = vec![(start, end)];
        if !t.cases.is_empty() {
            let (handler, catch_end) = (self.code.new_label(), self.code.new_label());
            self.code.add_handler(Handler { start, end, handler, catch_type: 0 });
            self.code.locals_release(protected);
            self.code.bind_handler(handler, throwable_vt);
            let mark = self.code.locals_mark();
            let e = self.store_new(&throwable);
            for case in &prog.cases[t.cases.range()] {
                let next = self.code.new_label();
                let case_mark = self.code.locals_mark();
                self.pattern(case.pat, e, &throwable, next);
                if let Some(g) = case.guard {
                    self.cond(g, next, false);
                }
                self.expr(case.body, want);
                if let Some(slot) = result {
                    self.store(slot, want);
                }
                self.code.locals_release(case_mark);
                self.goto(normal_exit);
                self.code.bind(next);
            }
            self.load(e, &throwable);
            self.code.op(op::ATHROW);
            self.code.pop();
            self.code.end_path();
            self.code.bind(catch_end);
            self.code.locals_release(mark);
            any_ranges.push((handler, catch_end));
        }
        if let Some(f) = t.finalizer {
            // A `return` inside the finalizer itself belongs to the enclosing frame.
            self.m.finalizers.pop();
            let any = self.code.new_label();
            for (from, to) in any_ranges {
                self.code.add_handler(Handler { start: from, end: to, handler: any, catch_type: 0 });
            }
            self.code.locals_release(protected);
            self.code.bind_handler(any, throwable_vt);
            let mark = self.code.locals_mark();
            let thrown = self.store_new(&throwable);
            self.statement(f);
            self.load(thrown, &throwable);
            self.code.op(op::ATHROW);
            self.code.pop();
            self.code.end_path();
            self.code.locals_release(mark);
            self.code.bind(normal_exit);
            self.statement(f);
            if let Some(pending) = pending {
                let skip = self.code.new_label();
                self.load(pending, &JType::I);
                self.jump_if(op::IFEQ, 1, skip);
                match self.m.finalizers.last().copied() {
                    Some(outer) => {
                        self.iconst(1);
                        self.store(outer.pending, &JType::I);
                        if let (Some(slot), Some(outer_slot)) = (return_slot, outer.value) {
                            self.load(slot, &ret);
                            self.store(outer_slot, &ret);
                        }
                        self.goto(outer.exit);
                    }
                    None => {
                        if let Some(slot) = return_slot {
                            self.load(slot, &ret);
                        }
                        self.return_value(&ret);
                    }
                }
                self.code.bind(skip);
            }
        } else {
            self.code.bind(normal_exit);
        }
        self.code.bind(after);
        self.reload_stack(spilled);
        if let Some(slot) = result {
            self.load(slot, want);
        }
        self.code.locals_release(outer);
    }

    /// Moves the operands on the stack into fresh locals, deepest first.
    fn spill_stack(&mut self) -> Vec<(u16, JType)> {
        let mut spilled = Vec::new();
        while let Some(&top) = self.code.stack.last() {
            let t = match top {
                VT::Int => JType::I,
                VT::Long => JType::J,
                VT::Float => JType::F,
                VT::Double => JType::D,
                VT::Object(_) | VT::Null => JType::object(),
                _ => {
                    self.unsupported("not supported on the JVM yet: try as an argument of a constructor");
                    return spilled;
                }
            };
            let slot = self.code.new_local(top);
            self.store(slot, &t);
            spilled.push((slot, t));
        }
        spilled.reverse();
        spilled
    }

    fn reload_stack(&mut self, spilled: Vec<(u16, JType)>) {
        for (slot, t) in spilled {
            self.load(slot, &t);
        }
    }

    /// The parameter type under which the JDK renders a value of type `t`: `String.valueOf`,
    /// `StringBuilder.append` and `insert` have an overload per primitive, one for a `String`
    /// and one for any other reference.
    pub fn rendered_as(t: &JType) -> JType {
        match t {
            JType::L(n) if &**n == STRING => t.clone(),
            JType::L(_) | JType::V => JType::object(),
            JType::B | JType::S => JType::I,
            p => p.clone(),
        }
    }

    pub fn string_value_of(&mut self, t: &JType) {
        let param = match t {
            JType::L(n) if &**n == STRING => return,
            JType::L(_) => JType::object(),
            JType::V => {
                self.sconst("()");
                return;
            }
            JType::B | JType::S => JType::I,
            p => p.clone(),
        };
        let string = JType::L(Rc::from(STRING));
        self.invoke(Invoke::Static, STRING, false, "valueOf", &[param], &string);
    }

    /// `a + b + c` on strings: one `invokedynamic` call of `makeConcatWithConstants`, as scalac
    /// writes it. Every operand of the chain is evaluated, left to right, before the call
    /// renders the first; the string literals are the text of the recipe.
    fn str_concat(&mut self, items: ListRef) -> JType {
        let prog = self.cx.input.prog;
        let mark = self.code.locals_mark();
        let mut operands = Vec::new();
        if let Some((&head, rest)) = prog.expr_list(items).split_first() {
            self.chain_head(head, &mut operands);
            operands.extend_from_slice(rest);
        }
        // scalac's typer folds the constants a chain starts with into one literal.
        let value_of = |e: TExprId| prog.expr(e).rendering().map_or(e, |(inner, _)| inner);
        let constant = |e: TExprId| matches!(prog.expr(value_of(e)), TExpr::Str(_) | TExpr::Int(_) | TExpr::Long(_) | TExpr::Bool(_) | TExpr::Char(_));
        let folded = match operands.iter().take_while(|&&e| constant(e)).count() {
            1 => 0,
            n => n,
        };
        let mut call = ConcatCall::default();
        let mut flushed = 0;
        for (i, &operand) in operands.iter().enumerate() {
            let value = value_of(operand);
            let text = match prog.expr(value) {
                TExpr::Str(text) => Some(prog.strings[text.0 as usize].clone()),
                TExpr::Int(v) if i < folded && self.static_type(value) == JType::I => Some(v.to_string()),
                TExpr::Long(v) if i < folded => Some(v.to_string()),
                TExpr::Bool(v) if i < folded => Some(v.to_string()),
                TExpr::Char(v) if i < folded => char::from_u32(v as u32).map(|c| c.to_string()),
                _ => None,
            };
            // scalac drops the empty literals before it counts.
            if text.as_deref() == Some("") {
                continue;
            }
            let string = JType::L(Rc::from(STRING));
            let t = match text {
                Some(_) => string.clone(),
                None => self.static_type(value),
            };
            // A value class's instance reaches the call as its box, whatever it holds.
            let boxed = text.is_none() && self.recorded_type(value).and_then(|rt| self.value_class_of(rt)).is_some();
            let param = match t {
                JType::L(ref n) if &**n == STRING && !boxed => t.clone(),
                JType::L(_) | JType::V => JType::object(),
                _ if boxed => JType::object(),
                ref p => p.clone(),
            };
            let slots = if matches!(param, JType::J | JType::D) { 2 } else { 1 };
            if call.slots + slots >= CONCAT_SLOTS {
                self.concat_call(std::mem::take(&mut call));
                flushed += 1;
            }
            match text {
                Some(text) if text.contains(['\u{1}', '\u{2}']) => call.constant(text),
                Some(text) if call.bytes + recipe_bytes(&text) > RECIPE_BYTES => {
                    self.sconst(&text);
                    call.argument(string, 1);
                }
                Some(text) => call.text(&text),
                None => {
                    if t == JType::V {
                        self.statement(value);
                        self.unit_value();
                    } else if boxed {
                        self.expr(value, &param);
                    } else {
                        self.expr(value, &t);
                    }
                    call.argument(param, slots);
                }
            }
        }
        let string = JType::L(Rc::from(STRING));
        self.concat_call(call);
        if flushed > 0 {
            let mut joined = ConcatCall::default();
            for _ in 0..=flushed {
                joined.argument(string.clone(), 1);
            }
            self.concat_call(joined);
        }
        self.code.locals_release(mark);
        string
    }

    /// The operands the head of a concatenation brings to its chain, as the IR says what the
    /// head is to it: a chain's own operands, those behind a block's result, whose statements
    /// run here, before the first operand, or the head as one operand.
    fn chain_head(&mut self, head: TExprId, operands: &mut Vec<TExprId>) {
        let prog = self.cx.input.prog;
        match prog.chain_head(head) {
            ChainHead::Operand => operands.push(head),
            ChainHead::Chain(items) => {
                if let Some((&inner, rest)) = prog.expr_list(items).split_first() {
                    self.chain_head(inner, operands);
                    operands.extend_from_slice(rest);
                }
            }
            ChainHead::Block(stmts, res) => {
                self.block_statements(stmts);
                self.chain_head(res, operands);
            }
        }
    }

    fn concat_call(&mut self, call: ConcatCall) {
        if call.arguments.is_empty() && call.constants.is_empty() {
            return self.sconst(&call.recipe);
        }
        let string = JType::L(Rc::from(STRING));
        let cp = &mut self.cw.cp;
        let factory = cp.method(
            "java/lang/invoke/StringConcatFactory",
            "makeConcatWithConstants",
            "(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/invoke/CallSite;",
        );
        let handle = cp.method_handle(6, factory);
        let mut constants = vec![cp.string(&call.recipe)];
        constants.extend(call.constants.iter().map(|c| cp.string(c)));
        let bootstrap = self.cw.bootstrap_method(handle, &constants);
        let site = self.cw.cp.invoke_dynamic(bootstrap, "makeConcatWithConstants", &method_desc(&call.arguments, &string));
        self.code.op_u16(op::INVOKEDYNAMIC, site);
        self.code.bytes.extend_from_slice(&[0, 0]);
        self.code.popn(call.arguments.len());
        let vt = self.vt(&string);
        self.code.push(vt);
    }

    // ---- locals, fields, statics ----

    pub fn load_this(&mut self) -> JType {
        if let (Some(u), Some(slot)) = (self.m.this_underlying.clone(), self.m.this_slot) {
            self.load(slot, &u);
            return u;
        }
        let t = match self.this_class {
            Some(c) => self.class_type(c),
            None => JType::object(),
        };
        match self.m.this_slot {
            Some(slot) => self.load(slot, &t),
            None => {
                self.unsupported("`this` outside of an instance");
                self.code.op(op::ACONST_NULL);
                self.code.push(VT::Null);
            }
        }
        t
    }

    fn captured_field(&self, s: SymId) -> Option<(String, JType, Held)> {
        self.class_captures.iter().find(|c| c.0 == s).map(|c| (c.1.clone(), c.2.clone(), c.3))
    }

    /// Whether the local lazy val `s` is held here as the function that forces it, which a
    /// class or a lifted method capturing it takes in place of the cell its own scope keeps.
    fn lazy_thunk(&self, s: SymId) -> bool {
        match self.m.locals.get(&s) {
            Some(l) => !l.cell,
            None => matches!(self.captured_field(s), Some((_, _, Held::Thunk))),
        }
    }

    /// Whether the local lives in a cell where the current code sees it.
    pub fn is_cell(&self, s: SymId) -> bool {
        match self.m.locals.get(&s) {
            Some(l) => l.cell,
            None => self.captured_field(s).map_or(false, |c| c.2 == Held::Cell),
        }
    }

    /// Pushes the cell of a local, or its value when it has none: what a closure captures.
    pub fn load_capture(&mut self, s: SymId) -> JType {
        let cell_type = JType::L(Rc::from(OBJECT_REF));
        if let Some(l) = self.m.locals.get(&s).cloned() {
            let t = if l.cell { cell_type } else { l.ty };
            self.load(l.slot, &t);
            return t;
        }
        if let Some((field, ty, held)) = self.captured_field(s) {
            self.load_this();
            let t = match held {
                Held::Value => ty,
                Held::Cell => cell_type,
                Held::Thunk => self.function_type(0),
            };
            let owner = self.this_name.clone();
            self.getfield(&owner, &field, &t);
            return t;
        }
        let name = self.cx.input.interner.get(self.cx.input.syms.sym(s).name).to_string();
        self.unsupported(&format!("the local {} is not in scope of the generated method", name));
        self.code.op(op::ACONST_NULL);
        self.code.push(VT::Null);
        JType::object()
    }

    fn load_local(&mut self, s: SymId) -> JType {
        let info = self.cx.input.syms.sym(s);
        if info.mods & mods::LAZY != 0 && info.kind != SymKind::Def {
            if self.lazy_thunk(s) {
                let ty = match self.captured_field(s) {
                    Some((_, ty, Held::Thunk)) => ty,
                    _ => self.sym_type(s),
                };
                let ft = self.load_capture(s);
                let JType::L(owner) = &ft else { unreachable!() };
                let object = JType::object();
                self.invoke(Invoke::Interface, owner, true, "apply", &[], &object);
                self.adapt(&object, &ty);
                return ty;
            }
            return self.lazy_read(s);
        }
        let declared = if info.by_name { self.function_type(0) } else { self.sym_type(s) };
        let t = self.load_capture(s);
        if self.is_cell(s) {
            let object = JType::object();
            self.getfield(OBJECT_REF, "elem", &object);
            self.adapt(&object, &declared);
            return declared;
        }
        t
    }

    fn store_local(&mut self, s: SymId, value: TExprId) {
        let declared = self.sym_type(s);
        if self.is_cell(s) {
            self.load_capture(s);
            let object = JType::object();
            self.expr(value, &declared);
            self.adapt(&declared, &object);
            self.putfield(OBJECT_REF, "elem", &object);
            return;
        }
        match self.m.locals.get(&s).cloned() {
            Some(l) => {
                self.expr(value, &l.ty);
                self.store(l.slot, &l.ty);
            }
            None => self.unsupported("assignment to a local that is not in scope"),
        }
    }

    /// Declares a local with the value on the stack (of the local's declared type).
    pub fn declare_local(&mut self, s: SymId, ty: JType, cell: bool) {
        if cell {
            let object = JType::object();
            self.adapt(&ty, &object);
            let tmp = self.store_new(&object);
            self.new_object(OBJECT_REF);
            self.load(tmp, &object);
            self.invoke(Invoke::Special, OBJECT_REF, false, "<init>", &[object], &JType::V);
            let cell_type = JType::L(Rc::from(OBJECT_REF));
            let slot = self.store_new(&cell_type);
            self.m.locals.insert(s, Local { slot, ty, cell: true });
        } else {
            let slot = self.store_new(&ty);
            self.m.locals.insert(s, Local { slot, ty, cell: false });
        }
    }

    /// `classOf[C]`: the class constant, or a primitive's `TYPE`.
    fn class_of(&mut self, e: TExprId, c: ClassId) -> JType {
        let class = JType::L(Rc::from("java/lang/Class"));
        let info = self.cx.input.syms.class(c);
        // `classOf[Array[T]]` is the array class of `T`'s erasure, which the node's type says.
        if info.kind == ClassKind::Builtin && self.cx.input.interner.get(info.name) == "Array" {
            let types = self.cx.input.types;
            let element = self.recorded_type(e).and_then(|t| match types.get(t) {
                Type::Class(_, args) => types.items(args).first().copied(),
                _ => None,
            }).and_then(|array| match types.get(array) {
                Type::Class(_, args) => types.items(args).first().copied(),
                _ => None,
            });
            let array = match element {
                Some(elem) => self.array_type(Some(elem)),
                None => JType::L(Rc::from(OBJECT_ARRAY)),
            };
            let JType::L(name) = &array else { unreachable!() };
            let i = self.cw.cp.class(name);
            self.ldc(i);
            let vt = self.vt(&class);
            self.code.push(vt);
            return class;
        }
        // An opaque type's class literal is its erasure's (scalac's `ClassTag` of one over `Int`
        // is `Int`): the underlying type's, which has no class of its own.
        if info.kind == ClassKind::Opaque {
            if let Some(under) = info.underlying {
                let erased = self.erase(under);
                match &erased {
                    JType::L(name) => {
                        let i = self.cw.cp.class(name);
                        self.ldc(i);
                        let t = VT::Object(self.cw.cp.class("java/lang/Class"));
                        self.code.push(t);
                    }
                    JType::V => self.getstatic("java/lang/Void", "TYPE", &class),
                    _ => self.getstatic(erased.box_class(), "TYPE", &class),
                }
                return class;
            }
        }
        let primitive = if info.kind == ClassKind::Builtin {
            match self.cx.input.interner.get(info.name) {
                "Int" => Some("java/lang/Integer"),
                "Long" => Some("java/lang/Long"),
                "Double" => Some("java/lang/Double"),
                "Float" => Some("java/lang/Float"),
                "Boolean" => Some("java/lang/Boolean"),
                "Char" => Some("java/lang/Character"),
                "Byte" => Some("java/lang/Byte"),
                "Short" => Some("java/lang/Short"),
                "Unit" => Some("java/lang/Void"),
                _ => None,
            }
        } else {
            None
        };
        if let Some(boxed) = primitive {
            self.getstatic(boxed, "TYPE", &class);
            return class;
        }
        let name = self.class_name(c);
        let i = self.cw.cp.class(&name);
        self.ldc(i);
        let t = VT::Object(self.cw.cp.class("java/lang/Class"));
        self.code.push(t);
        class
    }

    pub fn load_module(&mut self, c: ClassId) -> JType {
        // An object without a body that only its exports are reached through is not emitted,
        // and a Java class's statics have no object.
        if !self.cx.input.reach.classes[c.idx()] || self.cx.input.java_class(c).is_some() {
            return JType::V;
        }
        let name = self.class_name(c);
        let t = JType::L(name.clone());
        self.getstatic(&name, "MODULE$", &t);
        t
    }

    pub fn load_file_module(&mut self, file: crate::source::FileId) -> Rc<str> {
        let name: Rc<str> = Rc::from(self.cx.file_modules[file.0 as usize].as_str());
        let t = JType::L(name.clone());
        self.getstatic(&name, "MODULE$", &t);
        name
    }

    pub fn static_value(&mut self, s: SymId) -> JType {
        self.load_static(s)
    }

    /// A given object of a package or of objects, read as scalac reads it: its module, as a
    /// value of the given's declared type, which its class implements, so that a build over the
    /// object's products, which may not see its class's parents, casts no more than this one.
    fn given_object_read(&mut self, s: SymId) -> Option<JType> {
        let &k = self.cx.given_objects.get(&s)?;
        let name = self.class_name(k);
        self.getstatic(&name, "MODULE$", &JType::L(name.clone()));
        let declared = self.sym_type(s);
        let vt = self.vt(&declared);
        self.code.retype_top(vt);
        Some(declared)
    }

    fn load_static(&mut self, s: SymId) -> JType {
        let cx = self.cx;
        if let Some(t) = self.given_object_read(s) {
            return t;
        }
        let info = cx.input.syms.sym(s);
        if let Some((cf, JMember::Field(i))) = cx.input.java_member(s) {
            let f = &cf.fields[i as usize];
            let t = parse_type(&f.descriptor).0;
            self.getstatic(&cf.name, &f.name, &t);
            return self.java_value(t);
        }
        if let SymKind::EnumValue(case) = info.kind {
            // A jar enum's value is a static field of its companion, as scalac writes it, and so
            // is a program enum's whose companion this build writes with the fields.
            if let Owner::Class(companion) = info.owner {
                if let Some(e) = cx.input.syms.class(companion).companion.filter(|&e| cx.linked_class(companion) || cx.has_enum_fields(e)) {
                    let owner = self.class_name(companion);
                    let t = self.class_type(e);
                    let name = encode(cx.input.interner.get(info.name));
                    self.getstatic(&owner, &name, &t);
                    return t;
                }
            }
            if let Owner::Class(companion) = info.owner {
                if cx.layout.has_body(companion) && cx.input.reach.classes[companion.idx()] {
                    let t = self.load_module(companion);
                    self.pop_value(&t);
                } else if let Some(e) = cx.enum_of_case(case) {
                    // The enum makes its values when it is initialised, so a read starts there:
                    // one that initialised the value's class first would wait for the enum, as
                    // its superclass, while another thread making the values waited for it.
                    let enum_name = self.class_name(e);
                    self.invoke(Invoke::Static, &enum_name, false, "$touch", &[], &JType::V);
                }
            }
            let name = self.class_name(case);
            let t = JType::L(name.clone());
            self.getstatic(&name, "$instance", &t);
            return t;
        }
        let m = self.mref(s);
        if m.kind == Invoke::Static && m.params.is_empty() && !matches!(info.kind, SymKind::Def) {
            self.getstatic(&m.owner, &m.name, &m.ret);
            return m.ret.clone();
        }
        match info.owner {
            Owner::Class(c) => {
                self.load_module(c);
            }
            _ => {
                self.load_file_module(info.file);
            }
        }
        self.invoke_mref(&m);
        m.ret.clone()
    }

    fn receiver(&mut self, r: TExprId, s: SymId) {
        let cx = self.cx;
        match cx.input.syms.sym(s).owner {
            Owner::Class(c) => {
                let t = self.class_type(c);
                self.expr(r, &t);
            }
            _ => {
                let object = JType::object();
                self.expr(r, &object);
            }
        }
    }

    fn field_read(&mut self, r: TExprId, s: SymId) -> JType {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        if cx.input.syms.js_member(s) {
            self.unsupported("a member of a JavaScript type");
            return JType::V;
        }
        if let Some((cf, JMember::Field(i))) = cx.input.java_member(s) {
            let f = &cf.fields[i as usize];
            let t = parse_type(&f.descriptor).0;
            if f.access & ACC_STATIC != 0 {
                self.getstatic(&cf.name, &f.name, &t);
            } else {
                let owner = JType::L(Rc::from(cf.name.as_str()));
                self.expr(r, &owner);
                self.getfield(&cf.name, &f.name, &t);
            }
            return self.java_value(t);
        }
        if let SymKind::Object(c) = info.kind {
            self.statement(r);
            return self.load_module(c);
        }
        // A given object through its owner: the owner is not initialised for it, as under scalac.
        if cx.given_objects.contains_key(&s) {
            if !matches!(cx.input.prog.expr(r), TExpr::Module(_) | TExpr::This) {
                self.statement(r);
            }
            return self.given_object_read(s).unwrap_or(JType::V);
        }
        if let Some(t) = self.vc_field_read(r, s) {
            return t;
        }
        if self.m.before_super && matches!(cx.input.prog.expr(r), TExpr::This) && self.m.locals.contains_key(&s) {
            return self.load_local(s);
        }
        // A val that a subclass may override is read through its accessor.
        let own_field = matches!(cx.input.prog.expr(r), TExpr::This)
            && matches!(info.owner, Owner::Class(c) if Some(c) == self.this_class
                && !cx.is_interface(c)
                && (info.mods & mods::PRIVATE != 0 || cx.input.syms.class(c).subclasses.is_empty() && (!cx.input.open_world || cx.input.syms.class(c).mods & mods::FINAL != 0)))
            && !cx.is_lazy_member(s)
            && info.kind != SymKind::Def
            && self.m.this_slot == Some(0)
            && self.this_class.map_or(false, |c| *cx.class_names[c.idx()] == *self.this_name);
        if own_field {
            let t = self.sym_type(s);
            let name = self.field_name(s);
            self.load_this();
            let owner = self.this_name.clone();
            self.getfield(&owner, &name, &t);
            return t;
        }
        // A parameter of no accessor, read by its class's own code alone (`Cx::private_params`).
        if let (true, Owner::Class(c)) = (cx.private_params.contains_key(&s), info.owner) {
            let t = self.sym_type(s);
            let name = self.field_name(s);
            let owner = self.class_name(c);
            self.expr(r, &JType::L(owner.clone()));
            self.getfield(&owner, &name, &t);
            return t;
        }
        let s = self.deferred_implementation(r, s).unwrap_or(s);
        self.receiver(r, s);
        let m = self.mref(s);
        self.invoke_mref(&m);
        m.ret.clone()
    }

    /// The implementation a program class was given for the deferred given `s`
    /// (`TClass::deferred_givens`) where the receiver `r` is an instance of that class or of a
    /// subclass: read on the class, as a class read from its module's products is, whose pickle
    /// holds the implementation as its member, so that a whole build and a split one agree.
    fn deferred_implementation(&self, r: TExprId, s: SymId) -> Option<SymId> {
        let cx = self.cx;
        if cx.input.syms.sym(s).mods & mods::DEFERRED == 0 {
            return None;
        }
        let mut c = match cx.input.types.get(self.recorded_type(r)?) {
            Type::This(c) | Type::Class(c, _) => c,
            _ => return None,
        };
        loop {
            let i = cx.tclass_of[c.idx()];
            if i != u32::MAX {
                if let Some(&(_, implementation)) = cx.input.prog.classes[i as usize].deferred_givens.iter().find(|&&(d, _)| d == s) {
                    return Some(implementation);
                }
            }
            c = cx.input.syms.class(c).superclass?;
        }
    }

    /// In link mode, the parameter of a value class read on a value of it is the value itself.
    fn vc_field_read(&mut self, r: TExprId, s: SymId) -> Option<JType> {
        let cx = self.cx;
        let Owner::Class(c) = cx.input.syms.sym(s).owner else { return None };
        if !cx.input.syms.class(c).value_class {
            return None;
        }
        let field = cx.input.syms.class(c).ctor.first().and_then(|cl| cl.params.first()).map(|p| p.sym)?;
        if field != s && cx.input.syms.class(c).members.get(&cx.input.syms.sym(s).name) != Some(&field) {
            return None;
        }
        if matches!(cx.input.prog.expr(r), TExpr::This) && Some(c) == self.this_class && self.m.this_underlying.is_none() {
            return None;
        }
        let u = self.vc_underlying(c);
        // `missing[V].u` is the box's accessor on the reference, as scalac casts the qualifier to
        // the class and selects (`Erasure.scala` 733-737): a null throws. A block's result is
        // the qualifier, where a conditional's, a match's or a `try`'s branches are each adapted
        // to the value class, a null to its zero.
        let mut target = r;
        while let TExpr::Block(_, res) = cx.input.prog.expr(target) {
            target = res;
        }
        let outer = self.vc_select.replace(target);
        self.expr(r, &u);
        self.vc_select = outer;
        Some(u)
    }

    fn assign(&mut self, target: TExprId, value: TExprId) {
        let cx = self.cx;
        if let TExpr::Field(_, s) | TExpr::Static(s) = cx.input.prog.expr(target) {
            if let Some((cf, JMember::Field(i))) = cx.input.java_member(s) {
                return self.java_field_write(target, &cf.name, &cf.fields[i as usize], value);
            }
        }
        match cx.input.prog.expr(target) {
            TExpr::Local(s) => self.store_local(s, value),
            TExpr::Static(s) => {
                let info = cx.input.syms.sym(s);
                match info.owner {
                    Owner::Class(c) => {
                        self.load_module(c);
                    }
                    _ => {
                        self.load_file_module(info.file);
                    }
                }
                self.setter_call(s, value);
            }
            TExpr::Field(r, s) => {
                self.receiver(r, s);
                self.setter_call(s, value);
            }
            _ => self.unsupported("assignment to this kind of target"),
        }
    }

    fn java_field_write(&mut self, target: TExprId, owner: &str, f: &crate::classfile::Field, value: TExprId) {
        let t = parse_type(&f.descriptor).0;
        if self.java_value_type(t.clone()) != t {
            self.unsupported("assignment to a Java array field outside link mode");
            return;
        }
        if f.access & ACC_STATIC != 0 {
            self.expr(value, &t);
            self.putstatic(owner, &f.name, &t);
        } else {
            if let TExpr::Field(r, _) = self.cx.input.prog.expr(target) {
                self.expr(r, &JType::L(Rc::from(owner)));
            }
            self.expr(value, &t);
            self.putfield(owner, &f.name, &t);
        }
    }

    fn setter_call(&mut self, s: SymId, value: TExprId) {
        let m = self.mref(s);
        self.expr(value, &m.ret);
        let name = format!("{}_$eq", m.name);
        self.invoke(m.kind, &m.owner, m.owner_is_interface, &name, &[m.ret.clone()], &JType::V);
    }

    // ---- calls ----

    /// The arguments of a call against the parameter types. A `()` standing in for a parameter
    /// with a default is the call of the default's getter, which takes the arguments before it.
    fn arguments(&mut self, callee: Option<SymId>, args: ListRef, params: &[JType], getter: &dyn Fn(&mut Self, usize, &[(u16, JType)])) {
        let cx = self.cx;
        let defaults: Vec<bool> = match callee.and_then(|s| cx.input.syms.sym(s).info.sig.as_ref()) {
            Some(sig) => sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.has_default)).collect(),
            None => Vec::new(),
        };
        // A right-associative extension takes its arguments in another order than they are
        // written and evaluated: evaluated in theirs into locals, then loaded in its.
        let order = callee.and_then(|s| self.jvm_param_order(s)).filter(|o| o.len() == params.len());
        let Some(order) = order else {
            self.arguments_with(&defaults, args, params, getter);
            return;
        };
        let mut written = vec![JType::object(); params.len()];
        for (j, &k) in order.iter().enumerate() {
            written[k] = params[j].clone();
        }
        self.arguments_with(&defaults, args, &written, getter);
        let mut slots = vec![0u16; written.len()];
        for k in (0..written.len()).rev() {
            slots[k] = self.store_new(&written[k]);
        }
        for &k in &order {
            self.load(slots[k], &written[k]);
        }
    }

    /// The arguments an enum case passes to the constructor of its enum.
    pub fn super_arguments(&mut self, enum_class: ClassId, args: ListRef, params: &[JType]) {
        self.ctor_arguments(enum_class, args, params);
    }

    /// The arguments of the constructor of `c`, a left-out one through its default getter; a
    /// class of the JDK has no getters, so its default is evaluated where it stands.
    pub fn ctor_arguments(&mut self, c: ClassId, args: ListRef, params: &[JType]) {
        self.ctor_arguments_of(c, None, args, params);
    }

    /// The same for the secondary constructor `via` of `c` when one is named.
    pub fn ctor_arguments_of(&mut self, c: ClassId, via: Option<SymId>, args: ListRef, params: &[JType]) {
        let cx = self.cx;
        let flags: Vec<bool> = match via {
            Some(s) => cx.input.syms.sym(s).sig.as_ref().map_or(Vec::new(), |sig| {
                sig.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.has_default)).collect()
            }),
            None => cx.input.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.has_default)).collect(),
        };
        let types = params.to_vec();
        if cx.jvm_class(c).is_some() {
            let defaults: Vec<Option<TExprId>> = match (via, cx.tclass_of.get(c.idx()).copied().filter(|&i| i != u32::MAX)) {
                (Some(s), _) => cx.input.prog.funs.iter().find(|f| f.sym == s).map_or(Vec::new(), |f| f.defaults.clone()),
                (None, Some(i)) => cx.input.prog.classes[i as usize].ctor_defaults[self.capture_count(c)..].to_vec(),
                (None, None) => Vec::new(),
            };
            return self.arguments_with(&flags, args, params, &|g: &mut Self, i, _| match defaults.get(i).copied().flatten() {
                Some(d) => g.expr(d, &types[i]),
                None => g.adapt(&JType::V, &types[i]),
            });
        }
        let owner = self.class_name(c);
        // A class whose companion holds the getters (scalac's layout): each takes the parameters
        // of the clauses before the default's.
        // Per parameter, where its clause starts and the type its getter returns: a by-name
        // parameter's value (`companion_default_getters`).
        let shapes: Option<Vec<(usize, Option<TypeId>)>> = cx.ctor_defaults_on_companion(c).then(|| {
            let clauses: Vec<ClauseSig> = match via {
                Some(s) => cx.input.syms.sym(s).sig.as_ref().map_or(Vec::new(), |sig| sig.clauses.clone()),
                None => cx.input.syms.class(c).ctor.clone(),
            };
            let mut out = Vec::new();
            let mut start = 0;
            for cl in &clauses {
                out.extend(cl.params.iter().map(|p| (start, p.by_name.then_some(p.ty))));
                start += cl.params.len();
            }
            out
        });
        self.arguments_with(&flags, args, params, &|g: &mut Self, i, temps| {
            let name = format!("$lessinit$greater$default${}", i + 1);
            if let Some(shapes) = &shapes {
                let (before, by_name) = shapes.get(i).copied().unwrap_or((i, None));
                let module: Rc<str> = Rc::from(g.cx.companion_name(c));
                g.getstatic(&module, "MODULE$", &JType::L(module.clone()));
                let ret = match by_name {
                    Some(t) => g.erase(t),
                    None => types[i].clone(),
                };
                let desc = method_desc(&types[..before], &ret);
                g.getter_call(Invoke::Virtual, &module, false, name, &desc, temps, &types[i], by_name.is_some());
                return;
            }
            if g.jar_ctor_default(c, &name, temps, &types[i]) {
                return;
            }
            for (slot, t) in temps {
                g.load(*slot, t);
            }
            let before: Vec<JType> = temps.iter().map(|(_, t)| t.clone()).collect();
            let interface = g.cx.is_interface(c);
            g.invoke(Invoke::Static, &owner, interface, &name, &before, &types[i]);
        });
    }

    /// A jar class's constructor default as scalac's class files have it: the companion's getter,
    /// taking the arguments of the clauses before the default's only, called on the companion as
    /// scalac calls it, or through the static forwarder of the class where the companion's class
    /// file is not read.
    fn jar_ctor_default(&mut self, c: ClassId, name: &str, temps: &[(u16, JType)], want: &JType) -> bool {
        let cx = self.cx;
        let companion = cx.input.syms.class(c).companion;
        let forwarder = cx.class_files.get(&c).and_then(|cf| cf.methods.iter().find(|m| m.name == name && m.access & ACC_STATIC != 0)).map(|m| m.descriptor.clone());
        let on_companion = || {
            let k = companion?;
            let cf = cx.class_files.get(&k)?;
            cf.methods.iter().find(|m| m.name == name && m.access & ACC_STATIC == 0).map(|m| (Rc::from(cf.name.as_str()), m.descriptor.clone()))
        };
        let (kind, owner, desc) = match (on_companion(), forwarder) {
            (Some((module, desc)), _) => {
                self.getstatic(&module, "MODULE$", &JType::L(Rc::clone(&module)));
                (Invoke::Virtual, module, desc)
            }
            (None, Some(desc)) => (Invoke::Static, self.class_name(c), desc),
            (None, None) => return false,
        };
        let (params, ret) = parse_method_desc(&desc);
        for ((slot, t), p) in temps.iter().zip(&params) {
            self.load(*slot, t);
            self.adapt(t, p);
        }
        self.invoke_desc(kind, &owner, false, name, &desc, params.len(), &ret);
        self.adapt(&ret, want);
        true
    }

    /// A parameter without an argument, written or not, takes its default.
    fn arguments_with(&mut self, defaults: &[bool], args: ListRef, params: &[JType], getter: &dyn Fn(&mut Self, usize, &[(u16, JType)])) {
        let cx = self.cx;
        let written = cx.input.prog.expr_list(args);
        let n = written.len().max(params.len().min(defaults.len()));
        let items: Vec<Option<TExprId>> = (0..n).map(|i| written.get(i).copied()).collect();
        let uses_default = |i: usize, a: Option<TExprId>| {
            defaults.get(i).copied().unwrap_or(false) && a.map_or(true, |a| matches!(cx.input.prog.expr(a), TExpr::Unit))
        };
        if !items.iter().enumerate().any(|(i, &a)| uses_default(i, a)) {
            for (i, &a) in items.iter().enumerate() {
                let want = params.get(i).cloned().unwrap_or_else(JType::object);
                if let Some(a) = a {
                    if let JType::L(n) = &want {
                        if n.starts_with('[') {
                            let n = n.to_string();
                            self.java_array_argument(a, &n);
                            continue;
                        }
                    }
                    self.expr(a, &want);
                }
            }
            return;
        }
        // The arguments that are written run first, as they do at a JS call site; the defaults
        // follow in parameter order, each seeing the parameters before it.
        let mut temps: Vec<(u16, JType)> = Vec::new();
        for (i, &a) in items.iter().enumerate() {
            let want = params.get(i).cloned().unwrap_or_else(JType::object);
            match a {
                // A placeholder: the verifier wants every slot below a later one assigned, and
                // holding a value of the slot's type where a later argument branches (a null, not
                // the boxed unit, which no frame of a reference type admits).
                Some(a) if !uses_default(i, Some(a)) => self.expr(a, &want),
                _ if matches!(want, JType::L(_)) => {
                    self.code.op(op::ACONST_NULL);
                    self.code.push(VT::Null);
                }
                _ => self.adapt(&JType::V, &want),
            }
            let slot = self.store_new(&want);
            temps.push((slot, want));
        }
        for (i, &a) in items.iter().enumerate() {
            if uses_default(i, a) {
                getter(self, i, &temps[..i]);
                let (slot, want) = temps[i].clone();
                self.store(slot, &want);
            }
        }
        for (slot, t) in &temps {
            self.load(*slot, t);
        }
    }

    fn call_static(&mut self, s: SymId, args: ListRef) -> JType {
        let cx = self.cx;
        let info = cx.input.syms.sym(s);
        if let Some((_, JMember::Method(_))) = cx.input.java_member(s) {
            return self.call_java_static(s, args);
        }
        if let Some(t) = self.call_jdk_static(s, args) {
            return t;
        }
        if info.owner == Owner::Local {
            if info.kind != SymKind::Def {
                // A local function value applied through its name.
                let t = self.load_local(s);
                let n = cx.input.prog.expr_list(args).len();
                let ft = self.function_type(n);
                self.adapt(&t, &ft);
                let object = JType::object();
                for &a in cx.input.prog.expr_list(args) {
                    self.expr(a, &object);
                }
                let JType::L(owner) = &ft else { unreachable!() };
                self.invoke(Invoke::Interface, owner, true, "apply", &vec![JType::object(); n], &object);
                return object;
            }
            let lifted = self.lifted_fun(s);
            self.load_captures(&lifted);
            let params = lifted.params.clone();
            self.arguments(Some(s), args, &params, &|g: &mut Self, i, temps| g.local_default(s, i, temps));
            let owner = self.this_name.clone();
            let n = lifted.captures.len() + lifted.this as usize + lifted.params.len();
            self.invoke_desc(Invoke::Static, &owner, self.is_interface, &lifted.name, &lifted.desc, n, &lifted.ret);
            return lifted.ret.clone();
        }
        let m = self.mref(s);
        let module = self.load_file_module(info.file);
        let params = m.params.clone();
        self.arguments(Some(s), args, &params, &|g: &mut Self, i, temps| {
            let module_type = JType::L(module.clone());
            g.getstatic(&module, "MODULE$", &module_type);
            g.default_getter_call(s, i, temps);
        });
        self.invoke_mref(&m);
        m.ret.clone()
    }

    /// Calls `name$default$N` with the arguments of the clauses before the default's, the first
    /// of `temps`; the receiver is on the stack. A jar's getter is called as its class file
    /// declares it, the program's as `emit_fun` writes it, which is scalac's layout.
    fn default_getter_call(&mut self, s: SymId, i: usize, temps: &[(u16, JType)]) {
        let m = self.mref(s);
        // The parameter's type in the signature's order, the order of `temps`.
        let want = match self.jvm_param_order(s) {
            Some(order) => order.iter().position(|&k| k == i).map_or_else(|| m.params[i].clone(), |j| m.params[j].clone()),
            None => m.params[i].clone(),
        };
        let shape = self.default_getter_shape(s, i);
        let args = if self.linked_owner(s).is_some() { temps.to_vec() } else { self.getter_args(s, shape.before, temps) };
        let before: Vec<JType> = args.iter().map(|(_, t)| t.clone()).collect();
        let (name, desc) = self.default_getter_ref(s, i, &before);
        self.getter_call(m.kind, &m.owner, m.owner_is_interface, name, &desc, &args, &want, shape.by_name);
    }

    /// The name and descriptor of the getter of default `i` of method `s`: a jar's as its class
    /// file declares it, the program's as `emit_fun` writes it; `before` are the types of the
    /// parameters before the default, which a jar member without its getter takes.
    pub fn default_getter_ref(&mut self, s: SymId, i: usize, before: &[JType]) -> (String, String) {
        let m = self.mref(s);
        let shape = self.default_getter_shape(s, i);
        let name = format!("{}$default${}", self.source_method_name(s), shape.index);
        let jar = self.linked_owner(s).and_then(|cf| cf.methods.iter().find(|g| g.name == name && g.access & ACC_STATIC == 0)).map(|g| g.descriptor.clone());
        let desc = match jar {
            Some(desc) => desc,
            // A jar member without its getter: teq's erasure of it, as before.
            None if self.linked_owner(s).is_some() => method_desc(before, &m.params[i]),
            None => method_desc(&m.params[..shape.before], &shape.ret),
        };
        (name, desc)
    }

    /// The getter `name` of descriptor `desc` called with the first of `temps` as its arguments,
    /// the receiver on the stack unless it is static, leaving the value as the parameter's type
    /// `want`: a by-name parameter's default is passed as a thunk that calls the getter, as
    /// scalac's caller passes it, its value a function or not (`y: => (() => Long)`, whose getter
    /// and thunk both erase to `Function0`).
    fn getter_call(&mut self, kind: Invoke, owner: &Rc<str>, interface: bool, name: String, desc: &str, temps: &[(u16, JType)], want: &JType, by_name: bool) {
        let (params, ret) = parse_method_desc(desc);
        for ((slot, t), p) in temps.iter().zip(&params) {
            self.load(*slot, t);
            self.adapt(t, p);
        }
        let thunk = self.function_type(0);
        if *want == thunk && (by_name || ret != thunk) {
            let getter = JarGetter { kind, owner: owner.clone(), owner_is_interface: interface, name, desc: desc.to_string(), ret };
            self.jar_default_thunk(getter, &params);
            return;
        }
        self.invoke_desc(kind, owner, interface, &name, desc, params.len(), &ret);
        self.adapt(&ret, want);
    }

    /// The getter of the default of parameter `i` (flattened, in the signature's order) of program
    /// method `s`, as scalac writes it: the number in its name (`m$default$N`), how many
    /// parameters it takes, those of the clauses before the default's as the class file orders
    /// them (`m.params[..before]`), its result, the parameter's type, a by-name parameter's value
    /// rather than its thunk, and whether the parameter is by-name. A right-associative
    /// extension's are numbered and ordered as its declaration has them (`declared_clauses`):
    /// `$plus$colon$default$1()String` of `extension (n: Int) def +:(s: String = "x")`.
    pub fn default_getter_shape(&mut self, s: SymId, i: usize) -> GetterShape {
        let m = self.mref(s);
        let fallback = GetterShape { index: i + 1, before: i, ret: m.params[i].clone(), by_name: false };
        let Some(sig) = self.cx.input.syms.sym(s).info.sig.as_ref() else { return fallback };
        let mut start = 0;
        let mut found = None;
        for (ci, cl) in sig.clauses.iter().enumerate() {
            if i < start + cl.params.len() {
                found = Some((ci, i - start));
                break;
            }
            start += cl.params.len();
        }
        let Some((ci, k)) = found else { return fallback };
        let p = sig.clauses[ci].params[k].clone();
        let clauses = self.declared_clauses(s).unwrap_or_else(|| (0..sig.clauses.len()).collect());
        let pos = clauses.iter().position(|&c| c == ci).unwrap_or(ci);
        let before: usize = clauses[..pos].iter().map(|&c| sig.clauses[c].params.len()).sum();
        // A parameter whose type names the method's type parameters keeps its getter typed so
        // here and in the pickle, where scalac's has the default's type:
        // teq's typer infers the method's type arguments without the getter's.
        let ret = if p.by_name { self.erase(p.ty) } else { m.params[before + k].clone() };
        GetterShape { index: before + k + 1, before, ret, by_name: p.by_name }
    }

    /// The arguments of a default's getter among `temps`, the arguments before the default in the
    /// signature's order: those of the clauses before the default's as the class file orders them.
    pub fn getter_args(&self, s: SymId, before: usize, temps: &[(u16, JType)]) -> Vec<(u16, JType)> {
        match self.jvm_param_order(s) {
            Some(order) => order[..before.min(order.len())].iter().filter_map(|&k| temps.get(k).cloned()).collect(),
            None => temps.to_vec(),
        }
    }

    /// A static method of a Java class: no receiver, `invokestatic`.
    /// A member of a std object standing for a JDK class's statics: the static, no receiver.
    fn call_jdk_static(&mut self, s: SymId, args: ListRef) -> Option<JType> {
        let cx = self.cx;
        if !matches!(cx.input.syms.sym(s).owner, Owner::Class(c) if cx.jdk_statics(c)) {
            return None;
        }
        let m = self.mref(s);
        if m.kind != Invoke::Static {
            return None;
        }
        let params = m.params.clone();
        self.arguments(Some(s), args, &params, &|_, _, _| {});
        self.invoke_mref(&m);
        Some(m.ret.clone())
    }

    fn call_java_static(&mut self, s: SymId, args: ListRef) -> JType {
        let m = self.mref(s);
        let params = m.params.clone();
        self.arguments(Some(s), args, &params, &|_, _, _| {});
        self.invoke_mref(&m);
        self.java_value(m.ret.clone())
    }

    /// The value of a Java member as the program sees it: what the member leaves, a JVM array
    /// included.
    fn java_value(&mut self, t: JType) -> JType {
        t
    }

    /// The type `java_value` leaves for a Java member's declared type.
    fn java_value_type(&self, t: JType) -> JType {
        t
    }

    /// A Java array parameter takes the elements of a varargs literal as a JVM array.
    fn java_array(&mut self, items: ListRef, array: &str) {
        let elem = &array[1..];
        let elem_type = parse_type(elem).0;
        let items = self.cx.input.prog.expr_list(items).to_vec();
        self.iconst(items.len() as i32);
        let (store, atype): (u8, Option<u8>) = match elem_type {
            JType::Z => (op::BASTORE, Some(4)),
            JType::C => (op::CASTORE, Some(5)),
            JType::F => (op::FASTORE, Some(6)),
            JType::D => (op::DASTORE, Some(7)),
            JType::B => (op::BASTORE, Some(8)),
            JType::S => (op::SASTORE, Some(9)),
            JType::I => (op::IASTORE, Some(10)),
            JType::J => (op::LASTORE, Some(11)),
            _ => (op::AASTORE, None),
        };
        match atype {
            Some(code) => self.code.op_u8(op::NEWARRAY, code),
            None => {
                let class = elem.strip_prefix('L').and_then(|e| e.strip_suffix(';')).unwrap_or(elem);
                let index = self.cw.cp.class(class);
                self.code.op_u16(op::ANEWARRAY, index);
            }
        }
        self.code.pop();
        let array_type = JType::L(Rc::from(array));
        let vt = self.vt(&array_type);
        self.code.push(vt);
        for (i, item) in items.into_iter().enumerate() {
            self.dup();
            self.iconst(i as i32);
            self.expr(item, &elem_type);
            self.code.op(store);
            self.code.popn(3);
        }
    }

    /// The erasure of the elements of a varargs literal's sequence type; a value class's
    /// instances are its boxes there, whatever the class holds.
    fn seq_literal_element(&mut self, e: TExprId) -> JType {
        let types = self.cx.input.types;
        match self.recorded_type(e).map(|t| types.get(t)) {
            Some(Type::Class(_, args)) => match types.items(args).first() {
                Some(&elem) if self.value_class_of(elem).is_some() => JType::object(),
                Some(&elem) => self.erase(elem),
                None => JType::object(),
            },
            _ => JType::object(),
        }
    }

    /// An `Object[]` of the items boxed, each brought to `elem` first: an `Int` where the
    /// sequence's elements are `Double`s is a `Double`, which the typer leaves implicit.
    fn boxed_array(&mut self, items: ListRef, elem: &JType) {
        if elem.is_ref() {
            return self.java_array(items, OBJECT_ARRAY);
        }
        let items = self.cx.input.prog.expr_list(items).to_vec();
        let object = JType::object();
        self.iconst(items.len() as i32);
        let index = self.cw.cp.class(OBJECT);
        self.code.op_u16(op::ANEWARRAY, index);
        self.code.pop();
        let array_type = JType::L(Rc::from(OBJECT_ARRAY));
        let vt = self.vt(&array_type);
        self.code.push(vt);
        for (i, item) in items.into_iter().enumerate() {
            self.dup();
            self.iconst(i as i32);
            self.expr(item, elem);
            self.adapt(elem, &object);
            self.code.op(op::AASTORE);
            self.code.popn(3);
        }
    }

    /// An argument for a parameter that wants a JVM array (a Java varargs or array parameter).
    /// An array spliced in (`Stream.of(arr*)`) is passed itself, as scalac passes it, where its
    /// class is the parameter's; another sequence (`Stream.of(xs*)`) is copied into an array of
    /// the parameter's element class, as scalac's `scala.runtime.Arrays.seqToArray` does.
    fn java_array_argument(&mut self, a: TExprId, want: &str) {
        if let Some(array) = self.spliced_array(a, want) {
            let t = self.expr_static(array);
            self.adapt(&t, &JType::L(Rc::from(want)));
            return;
        }
        match self.cx.input.prog.expr(a) {
            TExpr::SeqLit(items) => self.java_array(items, want),
            _ => {
                let t = self.expr_static(a);
                if matches!(&t, JType::L(name) if !name.starts_with('[') && &**name != OBJECT) {
                    self.adapt(&t, &JType::L(Rc::from(SEQ)));
                    let class = JType::L(Rc::from("java/lang/Class"));
                    match parse_type(&want[1..]).0 {
                        JType::L(name) => {
                            let i = self.cw.cp.class(&name);
                            self.ldc(i);
                            let vt = self.vt(&class);
                            self.code.push(vt);
                        }
                        elem => self.getstatic(elem.box_class(), "TYPE", &class),
                    }
                    let desc = format!("(L{};Ljava/lang/Class;)Ljava/lang/Object;", SEQ);
                    self.invoke_desc(Invoke::Static, "scala/runtime/Arrays", false, "seqToArray", &desc, 2, &JType::object());
                    self.adapt(&JType::object(), &JType::L(Rc::from(want)));
                } else {
                    self.adapt(&t, &JType::L(Rc::from(want)));
                }
            }
        }
    }

    /// The array an argument `arr*` spliced into a Java array parameter stands for: the typer
    /// passes it through scala-library's conversion to a sequence (`copyArrayToImmutableIndexedSeq`,
    /// `wrapRefArray`, ..), whose argument it marks (`Program::spread_bits`); such a conversion
    /// written in the program is copied, as scalac copies it. The array goes as it is where the
    /// JVM takes it for the parameter: any array of references for an array of references, the
    /// typer having found its elements the parameter's (a `checkcast` where the class is not
    /// known to be), and a primitive array for the same.
    fn spliced_array(&mut self, a: TExprId, want: &str) -> Option<TExprId> {
        let cx = self.cx;
        let (s, args) = match cx.input.prog.expr(a) {
            TExpr::CallStatic(s, args) | TExpr::CallMethod(_, s, args) => (s, args),
            _ => return None,
        };
        let info = cx.input.syms.sym(s);
        let name = cx.input.interner.get(info.name);
        let conversion = name == "copyArrayToImmutableIndexedSeq" || name == "genericWrapArray" || (name.starts_with("wrap") && name.ends_with("Array"));
        let Owner::Class(owner) = info.owner else { return None };
        if !conversion || !matches!(&*self.class_name(owner), "scala/Predef$" | "scala/LowPriorityImplicits" | "scala/LowPriorityImplicits2") {
            return None;
        }
        let items = cx.input.prog.expr_list(args);
        let &[array] = items else { return None };
        if !cx.input.prog.is_spread(array) {
            return None;
        }
        let JType::L(class) = self.erase(self.recorded_type(array)?) else { return None };
        let references = |d: &str| d.starts_with("[L") || d.starts_with("[[");
        let fits = &*class == want || (references(want) && references(&class));
        fits.then_some(array)
    }

    fn call_method(&mut self, r: TExprId, s: SymId, args: ListRef) -> JType {
        let cx = self.cx;
        if cx.input.syms.js_member(s) {
            self.unsupported("a member of a JavaScript type");
            return JType::V;
        }
        if let Some(t) = self.call_extension(r, s, args) {
            return t;
        }
        if let Some(&(_, helper)) = cx.input.reach.interpolators.iter().find(|&&(m, _)| m == s) {
            return self.call_interpolator(r, s, helper, args);
        }
        // An object selected through its owner (a jar's `CanEqual.derived`) is its instance, not
        // a call.
        if let SymKind::Object(c) = cx.input.syms.sym(s).kind {
            self.statement(r);
            return self.load_module(c);
        }
        let mut m = self.mref(s);
        let java = cx.input.java_member(s).is_some();
        if java && m.kind == Invoke::Static {
            return self.call_java_static(s, args);
        }
        if let Some(t) = self.call_jdk_static(s, args) {
            return t;
        }
        if let TExpr::Super(target) = cx.input.prog.expr(r) {
            // `invokespecial` on the superclass, or on the interface of a trait of this class.
            let superclass = self.this_class.and_then(|c| cx.input.syms.class(c).superclass);
            let owner = match target {
                SuperTarget::Chain => superclass,
                SuperTarget::Class(c) => Some(c),
                SuperTarget::Mixin(_) => None,
            };
            match owner {
                // `super` inside a trait goes through the accessor its class implements.
                _ if matches!(target, SuperTarget::Mixin(_)) => {
                    let SuperTarget::Mixin(t) = target else { unreachable!() };
                    m = Rc::new(self.super_accessor_ref(t, s));
                }
                Some(c) if !matches!(target, SuperTarget::Class(_)) || cx.input.syms.class(c).kind == ClassKind::Trait => {
                    let mut special = (*m).clone();
                    special.owner = self.class_name(c);
                    special.owner_is_interface = cx.input.syms.class(c).kind == ClassKind::Trait;
                    special.kind = Invoke::Special;
                    m = Rc::new(special);
                }
                _ => self.unsupported("not supported on the JVM yet: this super call"),
            }
        }
        self.receiver(r, s);
        let params = m.params.clone();
        let uses_default = {
            let items = cx.input.prog.expr_list(args);
            let sig = cx.input.syms.sym(s).info.sig.as_ref();
            let flags: Vec<bool> = sig.map_or(Vec::new(), |sig| sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.has_default)).collect());
            items.iter().enumerate().any(|(i, &a)| flags.get(i).copied().unwrap_or(false) && matches!(cx.input.prog.expr(a), TExpr::Unit))
        };
        if uses_default {
            // The getter runs on the same receiver.
            let owner_type = JType::L(m.owner.clone());
            let recv = self.store_new(&owner_type);
            self.load(recv, &owner_type);
            self.arguments(Some(s), args, &params, &|g: &mut Self, i, temps| {
                g.load(recv, &owner_type);
                g.default_getter_call(s, i, temps);
            });
        } else {
            self.arguments(Some(s), args, &params, &|_, _, _| {});
        }
        self.invoke_mref(&m);
        if java {
            return self.java_value(m.ret.clone());
        }
        m.ret.clone()
    }

    /// In link mode, a method a value class declares is called as scalac calls it: the
    /// companion's `m$extension` on the underlying value, `C$.MODULE$.m$extension(u, args)`.
    fn call_extension(&mut self, r: TExprId, s: SymId, args: ListRef) -> Option<JType> {
        let cx = self.cx;
        let Owner::Class(c) = cx.input.syms.sym(s).owner else { return None };
        if !cx.input.syms.class(c).value_class || cx.input.syms.sym(s).kind != SymKind::Def {
            return None;
        }
        let u = self.vc_underlying(c);
        // A value class whose companion this build writes is called through it, as scalac and a
        // build over the class's products call it.
        if cx.vc_on_companion(c) {
            let mut own = vec![u.clone()];
            own.extend(self.erased_param_types(s));
            let ret = self.erased_return_type(s);
            let name = format!("{}$extension", self.method_name(s));
            let module: Rc<str> = Rc::from(cx.companion_name(c));
            let module_type = JType::L(module.clone());
            self.expr(r, &u);
            let recv = self.store_new(&u);
            self.getstatic(&module, "MODULE$", &module_type);
            self.load(recv, &u);
            let rest = own[1..].to_vec();
            // The default's getter takes the underlying value and the arguments of the clauses
            // before the default's; a by-name parameter's returns the value, which a thunk calling
            // the getter, its receiver and those arguments captured, passes (`getter_call`).
            self.arguments(Some(s), args, &rest, &|g: &mut Self, i, temps| {
                let shape = g.default_getter_shape(s, i);
                let (before, gret) = (shape.before, shape.ret);
                let getter = format!("{}$default${}$extension", g.source_method_name(s), shape.index);
                let mut gparams = vec![u.clone()];
                gparams.extend(rest[..before].iter().cloned());
                let mut gtemps = vec![(recv, u.clone())];
                gtemps.extend(temps.iter().take(before).cloned());
                g.getstatic(&module, "MODULE$", &module_type);
                g.getter_call(Invoke::Virtual, &module, false, getter, &method_desc(&gparams, &gret), &gtemps, &rest[i], shape.by_name);
            });
            self.invoke_desc(Invoke::Virtual, &module, false, &name, &method_desc(&own, &ret), own.len(), &ret);
            return Some(ret);
        }
        if cx.tclass_of[c.idx()] != u32::MAX && !cx.input.link.jar_classes.contains_key(&c) {
            let mut own = vec![u.clone()];
            own.extend(self.erased_param_types(s));
            let ret = self.erased_return_type(s);
            let base = self.method_name(s);
            let name = format!("{}$extension", base);
            let owner = self.class_name(c);
            self.expr(r, &u);
            let recv = self.store_new(&u);
            self.load(recv, &u);
            let rest = own[1..].to_vec();
            self.arguments(Some(s), args, &rest, &|g: &mut Self, i, temps| {
                g.load(recv, &u);
                g.vc_box(c, &u);
                let shape = g.default_getter_shape(s, i);
                let desc = method_desc(&rest[..shape.before], &shape.ret);
                let getter = format!("{}$default${}", g.source_method_name(s), shape.index);
                g.getter_call(Invoke::Virtual, &owner, false, getter, &desc, temps, &rest[i], shape.by_name);
            });
            self.invoke(Invoke::Static, &owner, false, &name, &own, &ret);
            return Some(ret);
        }
        let companion: &'a crate::classfile::ClassFile = cx.vc_companions.get(&c)?;
        let mut own = vec![u.clone()];
        own.extend(self.erased_param_types(s));
        let ret = self.erased_return_type(s);
        let name = format!("{}$extension", self.target_name(s).unwrap_or_else(|| encode(cx.input.interner.get(cx.input.syms.sym(s).name))));
        let arity = own.len();
        let m = pick_method(companion, &name, arity, &method_desc(&own, &ret))?;
        let (params, desc_ret) = parse_method_desc(&m.descriptor);
        let desc = m.descriptor.clone();
        let module = JType::L(Rc::from(companion.name.as_str()));
        self.expr(r, &u);
        let recv = self.store_new(&u);
        self.getstatic(&companion.name, "MODULE$", &module);
        self.load(recv, &u);
        self.adapt(&u, &params[0]);
        let rest: Vec<JType> = params[1..].iter().zip(&own[1..]).map(|(d, o)| if d == o { o.clone() } else { d.clone() }).collect();
        let base = match self.target_name(s) {
            Some(_) => encode(cx.input.interner.get(cx.input.syms.sym(s).name)),
            None => name.strip_suffix("$extension").unwrap_or(&name).to_string(),
        };
        // The default's getter is `m$default$N$extension`, on the underlying value and the
        // parameters of the clauses before the default's.
        self.arguments(Some(s), args, &rest, &|g: &mut Self, i, temps| {
            let getter = format!("{}$default${}$extension", base, i + 1);
            let Some(gm) = companion.methods.iter().find(|m| m.name == getter) else {
                g.unsupported("a default argument of a value class's extension method without its getter");
                g.adapt(&JType::V, &rest[i]);
                return;
            };
            let (gparams, gret) = parse_method_desc(&gm.descriptor);
            g.getstatic(&companion.name, "MODULE$", &module);
            g.load(recv, &u);
            g.adapt(&u, &gparams[0]);
            for ((slot, t), p) in temps.iter().zip(&gparams[1..]) {
                g.load(*slot, t);
                g.adapt(t, p);
            }
            g.invoke_desc(Invoke::Virtual, &companion.name, false, &getter, &gm.descriptor, gparams.len(), &gret);
            g.adapt(&gret, &rest[i]);
        });
        self.invoke_desc(Invoke::Virtual, &companion.name, false, &name, &desc, params.len(), &desc_ret);
        Some(if desc_ret == ret { ret } else { desc_ret })
    }

    /// `context.s(args)` of scala-library, which has no bytecode: the runtime's definition
    /// that lowers it as scalac does.
    fn call_interpolator(&mut self, r: TExprId, s: SymId, helper: SymId, args: ListRef) -> JType {
        let m = self.mref(helper);
        let params = m.params.clone();
        self.expr(r, &params[0]);
        self.arguments(Some(s), args, &params[1..], &|_, _, _| {});
        let name = self.cx.input.interner.get(self.cx.input.syms.sym(helper).name).to_string();
        self.helper_call(&name, 2)
    }

    /// `new C(args)` through the secondary constructor `s`: scalac's own `<init>` overload.
    fn new_via(&mut self, s: SymId, args: ListRef) -> JType {
        let cx = self.cx;
        let Owner::Class(c) = cx.input.syms.sym(s).owner else { return JType::V };
        let Some(name) = self.new_object_of(c) else { return JType::V };
        let params = self.param_types(s);
        self.ctor_arguments_of(c, Some(s), args, &params);
        self.invoke(Invoke::Special, &name, false, "<init>", &params, &JType::V);
        JType::L(name)
    }

    /// `new C` with its companion touched first, as an instance creation starts.
    fn new_object_of(&mut self, c: ClassId) -> Option<Rc<str>> {
        let cx = self.cx;
        let info = cx.input.syms.class(c);
        if info.js != JsKind::Scala {
            self.unsupported("a JavaScript class");
            return None;
        }
        if let Some(companion) = crate::emit::touched_companion(cx.input.syms, &cx.layout.has_body, c) {
            if cx.input.reach.classes[companion.idx()] {
                let t = self.load_module(companion);
                self.pop_value(&t);
            }
        }
        let name = self.class_name(c);
        self.new_object(&name);
        Some(name)
    }

    fn new_(&mut self, e: TExprId, c: ClassId, args: ListRef) -> JType {
        // A value class erases to its underlying value in link mode.
        if self.cx.input.syms.class(c).value_class {
            let u = match self.recorded_type(e).and_then(|rt| self.value_class_type(rt)) {
                Some((k, targs)) if k == c => self.vc_erasure(c, targs),
                _ => self.vc_underlying(c),
            };
            if let Some(&a) = self.cx.input.prog.expr_list(args).first() {
                // The argument takes the constructor parameter's own representation (dotty types it
                // against the erased parameter, `Erasure.scala` 827, boxing a value class there,
                // 394-398): a value class over a universal trait passed as `U` is its box, which is
                // then this class's underlying value.
                let param = self.vc_unmarked(&u);
                self.expr(a, &param);
                return u;
            }
        }
        let Some(name) = self.new_object_of(c) else { return JType::V };
        let params = self.new_arguments(c, args);
        self.invoke(Invoke::Special, &name, false, "<init>", &params, &JType::V);
        JType::L(name)
    }

    /// The superclass of an anonymous class, the secondary constructor its arguments go to,
    /// and the parameter types of that constructor.
    pub fn anon_super_params(&mut self, c: ClassId) -> Option<(ClassId, Option<SymId>, Vec<JType>)> {
        let cx = self.cx;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX || cx.input.syms.class(c).kind != ClassKind::Anon {
            return None;
        }
        let tc = &cx.input.prog.classes[idx as usize];
        tc.parent_args?;
        let sup = cx.input.syms.class(c).superclass?;
        let via = tc.parent_via;
        let types = match via {
            Some(v) => self.param_types(v),
            None if self.capture_count(sup) > 0 => {
                let mut types = self.anon_ctor_params(sup);
                types.extend(self.ctor_param_types(sup));
                types
            }
            None => self.ctor_param_types(sup),
        };
        Some((sup, via, types))
    }

    /// The arguments of a constructor of `c` with what it captures in front: the parameter
    /// types they were loaded as.
    pub fn new_arguments(&mut self, c: ClassId, args: ListRef) -> Vec<JType> {
        let info = self.cx.input.syms.class(c);
        let n_captures = self.capture_count(c);
        if n_captures > 0 || info.kind == ClassKind::Anon {
            if n_captures > 0 {
                self.load_new_captures(c, args, n_captures);
            }
            let mut params: Vec<JType> = self.anon_ctor_params(c);
            if let Some((sup, via, types)) = self.anon_super_params(c) {
                let rest = ListRef { start: args.start + n_captures as u32, len: args.len - n_captures as u32 };
                if via.is_none() && self.capture_count(sup) > 0 {
                    self.new_arguments(sup, rest);
                } else {
                    self.ctor_arguments_of(sup, via, rest, &types);
                }
                params.extend(types);
            }
            if info.kind != ClassKind::Anon {
                let rest = ListRef { start: args.start + n_captures as u32, len: args.len - n_captures as u32 };
                let declared = self.ctor_param_types(c);
                let captured = params.clone();
                self.ctor_arguments_after_captures(c, args, &captured, rest, &declared);
                params.extend(declared);
            }
            params
        } else if let Some(outer) = self.outer_of_unwritten(c, args) {
            // A class of a class that this build does not write (another module's, a jar's)
            // takes the enclosing instance the typer passes first, as scalac's `ExplicitOuter`
            // gives its constructor.
            // Loaded as the whole build loads a captured outer (`load_new_captures`).
            let first = self.cx.input.prog.expr_list(args)[0];
            self.expr_static(first);
            let rest = ListRef { start: args.start + 1, len: args.len - 1 };
            let declared = self.ctor_param_types(c);
            self.ctor_arguments(c, rest, &declared);
            std::iter::once(outer).chain(declared).collect()
        } else {
            let params = self.ctor_param_types(c);
            self.ctor_arguments(c, args, &params);
            params
        }
    }

    /// The type of the enclosing instance a `new` of `c` passes first where this build does not
    /// write `c`: a class or a given's class nested in a class or trait, made with one argument
    /// more than its constructor declares.
    fn outer_of_unwritten(&mut self, c: ClassId, args: ListRef) -> Option<JType> {
        let cx = self.cx;
        if cx.tclass_of[c.idx()] != u32::MAX {
            return None;
        }
        let info = cx.input.syms.class(c);
        let Owner::Class(o) = info.owner else { return None };
        if !matches!(info.kind, ClassKind::Class | ClassKind::GivenImpl) || matches!(cx.input.syms.class(o).kind, ClassKind::Object | ClassKind::Builtin) {
            return None;
        }
        let declared: usize = info.ctor.iter().map(|cl| cl.params.len()).sum();
        (args.len as usize == declared + 1).then(|| self.class_type(o))
    }

    /// Loads what a lifted class captures: the leading `n` of the arguments of its `new`.
    fn load_new_captures(&mut self, c: ClassId, args: ListRef, n: usize) {
        let cx = self.cx;
        let items = cx.input.prog.expr_list(args)[..n].to_vec();
        let passed = cx.input.prog.classes[cx.tclass_of[c.idx()] as usize].ctor_params[..n].iter().filter(|&&s| !self.is_local_def(s)).count();
        let through_defs = self.class_captured(c).split_off(passed);
        for a in items {
            match cx.input.prog.expr(a) {
                // A local def is not passed: the class gets a copy of the lifted method.
                TExpr::Local(s) if cx.input.syms.sym(s).kind == SymKind::Def && cx.input.syms.sym(s).owner == Owner::Local => {}
                TExpr::Local(s) if self.is_capturable(s) => {
                    self.load_capture(s);
                }
                // A lazy val this scope holds as its function is passed on as it is.
                TExpr::Local(s) if cx.input.syms.sym(s).owner == Owner::Local && self.lazy_thunk(s) => {
                    self.load_capture(s);
                }
                TExpr::Local(s) if cx.input.syms.sym(s).owner == Owner::Local && cx.input.syms.sym(s).kind != SymKind::Def => {
                    self.lambda(ListRef::EMPTY, a);
                }
                _ => {
                    if self.expr_static(a) == JType::V {
                        self.unit_value();
                    }
                }
            }
        }
        for s in through_defs {
            self.load_capture(s);
        }
    }

    /// The declared arguments of a named local class's constructor, a left-out one through its
    /// default getter, which takes the captures in front of the arguments before it.
    fn ctor_arguments_after_captures(&mut self, c: ClassId, args: ListRef, captured: &[JType], rest: ListRef, params: &[JType]) {
        let cx = self.cx;
        let flags: Vec<bool> = cx.input.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.has_default)).collect();
        let owner = self.class_name(c);
        let n = self.capture_count(c);
        let types = params.to_vec();
        self.arguments_with(&flags, rest, params, &|g: &mut Self, i, temps| {
            g.load_new_captures(c, args, n);
            let mut before: Vec<JType> = captured.to_vec();
            for (slot, t) in temps {
                g.load(*slot, t);
                before.push(t.clone());
            }
            g.invoke(Invoke::Static, &owner, false, &format!("$lessinit$greater$default${}", n + i + 1), &before, &types[i]);
        });
    }

    fn is_capturable(&self, s: SymId) -> bool {
        let info = self.cx.input.syms.sym(s);
        info.owner == Owner::Local && !(info.mods & mods::LAZY != 0 && info.kind != SymKind::Def)
    }

    pub fn ctor_param_types(&mut self, c: ClassId) -> Vec<JType> {
        let cx = self.cx;
        let params: Vec<SymId> = cx.input.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
        let own: Vec<JType> = params.into_iter().map(|p| self.sym_type(p)).collect();
        // A jar class's constructor is called as its class file declares it.
        let Some(cf) = cx.class_files.get(&c) else { return own };
        match pick_method(cf, "<init>", own.len(), &method_desc(&own, &JType::V)) {
            Some(m) => parse_method_desc(&m.descriptor).0.into_iter().zip(&own).map(|(d, o)| if d == *o { o.clone() } else { d }).collect(),
            None => own,
        }
    }

    /// How many leading constructor arguments of `c` are captured locals: all of an anonymous
    /// class's, the leading ones of a named local class.
    pub fn capture_count(&self, c: ClassId) -> usize {
        let cx = self.cx;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX {
            return 0;
        }
        let tc = &cx.input.prog.classes[idx as usize];
        if cx.input.syms.class(c).kind == ClassKind::Anon { tc.ctor_params.len() } else { tc.captures }
    }

    /// The constructor of a lifted class takes what its body captured first: a `var` as its cell.
    pub fn anon_ctor_params(&mut self, c: ClassId) -> Vec<JType> {
        let cx = self.cx;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX {
            return Vec::new();
        }
        self.class_captured(c).into_iter().map(|s| self.anon_capture_type(s)).collect()
    }

    /// The locals a lifted class holds in fields: what its body names, a local def left out
    /// since the class gets a copy of it, and in its place what that def captures.
    pub fn class_captured(&mut self, c: ClassId) -> Vec<SymId> {
        let cx = self.cx;
        let idx = cx.tclass_of[c.idx()];
        if idx == u32::MAX {
            return Vec::new();
        }
        let n = self.capture_count(c);
        let named = &cx.input.prog.classes[idx as usize].ctor_params[..n];
        let mut out: Vec<SymId> = named.iter().copied().filter(|&s| !self.is_local_def(s)).collect();
        let defs: Vec<SymId> = named.iter().copied().filter(|&s| self.is_local_def(s)).collect();
        for s in defs {
            let free = self.an.of_fun(cx, s);
            for &f in &free.syms {
                if !out.contains(&f) && !self.is_local_def(f) {
                    out.push(f);
                }
            }
        }
        out
    }

    pub fn is_local_def(&self, s: SymId) -> bool {
        let info = self.cx.input.syms.sym(s);
        info.kind == SymKind::Def && info.owner == Owner::Local
    }

    pub fn held(&self, s: SymId) -> Held {
        let info = self.cx.input.syms.sym(s);
        if info.owner != Owner::Local {
            Held::Value
        } else if info.kind == SymKind::Var {
            Held::Cell
        } else if info.mods & mods::LAZY != 0 && info.kind != SymKind::Def {
            Held::Thunk
        } else {
            Held::Value
        }
    }

    pub fn anon_capture_type(&mut self, s: SymId) -> JType {
        let info = self.cx.input.syms.sym(s);
        if self.held(s) == Held::Cell {
            JType::L(Rc::from(OBJECT_REF))
        } else if self.held(s) == Held::Thunk {
            self.function_type(0)
        } else if info.by_name {
            self.function_type(0)
        } else {
            self.sym_type(s)
        }
    }

    // ---- lambdas and lifted definitions ----

    fn fresh_name(&mut self, base: &str) -> String {
        self.counter += 1;
        format!("{}${}", base, self.counter)
    }

    fn make_lifted(&mut self, name: String, free: &Free, params: Vec<JType>, ret: JType) -> Rc<Lifted> {
        let mut captures = Vec::new();
        for &s in &free.syms {
            let info = self.cx.input.syms.sym(s);
            if info.kind == SymKind::Def {
                continue;
            }
            let lazy = info.mods & mods::LAZY != 0;
            let thunk = lazy && self.lazy_thunk(s);
            let cell = self.is_cell(s) || (lazy && !thunk);
            let ty = if info.by_name || thunk { self.function_type(0) } else { self.sym_type(s) };
            captures.push(Capture { sym: s, cell, ty });
        }
        let mut all: Vec<JType> = Vec::new();
        if free.this {
            all.push(match self.this_class {
                Some(c) => self.class_type(c),
                None => JType::object(),
            });
        }
        for c in &captures {
            all.push(if c.cell { JType::L(Rc::from(OBJECT_REF)) } else { c.ty.clone() });
        }
        all.extend(params.iter().cloned());
        let desc = method_desc(&all, &ret);
        Rc::new(Lifted { name, desc, this: free.this, captures, params, ret })
    }

    fn load_captures(&mut self, lifted: &Lifted) {
        if lifted.this {
            let t = self.load_this();
            if let (Some(u), Some(c)) = (self.m.this_underlying.clone(), self.this_class) {
                self.adapt(&t, &u);
                self.vc_box(c, &u);
            }
        }
        for c in &lifted.captures {
            let info = self.cx.input.syms.sym(c.sym);
            if info.mods & mods::LAZY != 0 && info.kind != SymKind::Def && c.cell {
                self.load_lazy_cell(c.sym);
            } else {
                self.load_capture(c.sym);
            }
        }
    }

    fn lambda(&mut self, params: ListRef, body: TExprId) -> JType {
        let cx = self.cx;
        let params: Vec<SymId> = cx.input.prog.sym_list(params).to_vec();
        let free = self.an.of_lambda(cx, &params, body);
        let name = self.fresh_name("$anonfun");
        let object = JType::object();
        let lifted = self.make_lifted(name, &free, vec![object.clone(); params.len()], object);
        self.load_captures(&lifted);
        let ft = self.function_type(params.len());
        self.pending.push(Pending { lifted: lifted.clone(), body: PendingBody::Lambda(params.clone(), body) });
        self.lambda_call_site(&lifted, &ft, "apply");
        ft
    }

    /// The receiver (unless the getter is static) and its arguments are on the stack: they
    /// become a `Function0` whose `apply` calls the getter.
    fn jar_default_thunk(&mut self, getter: JarGetter, params: &[JType]) {
        let mut captured: Vec<JType> = Vec::new();
        if getter.kind != Invoke::Static {
            captured.push(JType::L(getter.owner.clone()));
        }
        captured.extend(params.iter().cloned());
        let name = self.fresh_name("$default$thunk");
        let object = JType::object();
        let desc = method_desc(&captured, &object);
        let lifted = Rc::new(Lifted { name, desc, this: false, captures: Vec::new(), params: Vec::new(), ret: object });
        self.pending.push(Pending { lifted: lifted.clone(), body: PendingBody::JarGetter(getter) });
        let ft = self.function_type(0);
        self.call_site(&lifted, &ft, "apply", &captured);
    }

    /// `invokedynamic` through `LambdaMetafactory`: the captures on the stack become an instance
    /// of the function interface whose `apply` is the lifted method.
    /// The type of `this` a lifted method takes: the class's whose code it is, the box of a value
    /// class whose body its companion holds (`extension_bodies`) too.
    pub fn lifted_this_type(&mut self) -> JType {
        match self.this_class {
            Some(c) => self.class_type(c),
            None => JType::L(self.this_name.clone()),
        }
    }

    fn lambda_call_site(&mut self, lifted: &Lifted, interface: &JType, method: &str) {
        let mut captured: Vec<JType> = Vec::new();
        if lifted.this {
            let this = self.lifted_this_type();
            captured.push(this);
        }
        for c in &lifted.captures {
            captured.push(if c.cell { JType::L(Rc::from(OBJECT_REF)) } else { c.ty.clone() });
        }
        self.call_site(lifted, interface, method, &captured);
    }

    fn call_site(&mut self, lifted: &Lifted, interface: &JType, method: &str, captured: &[JType]) {
        let n = lifted.params.len();
        let sam = method_desc(&vec![JType::object(); n], &JType::object());
        let owner = self.this_name.clone();
        let cp = &mut self.cw.cp;
        let impl_ref = if self.is_interface {
            cp.interface_method(&owner, &lifted.name, &lifted.desc)
        } else {
            cp.method(&owner, &lifted.name, &lifted.desc)
        };
        let impl_handle = cp.method_handle(6, impl_ref);
        let sam_type = cp.method_type(&sam);
        let factory = cp.method(
            "java/lang/invoke/LambdaMetafactory",
            "metafactory",
            "(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/invoke/MethodType;Ljava/lang/invoke/MethodHandle;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;",
        );
        let factory_handle = cp.method_handle(6, factory);
        let bootstrap = self.cw.bootstrap_method(factory_handle, &[sam_type, impl_handle, sam_type]);
        let site_desc = method_desc(captured, interface);
        let site = self.cw.cp.invoke_dynamic(bootstrap, method, &site_desc);
        self.code.op_u16(op::INVOKEDYNAMIC, site);
        self.code.bytes.extend_from_slice(&[0, 0]);
        self.code.popn(captured.len());
        let vt = self.vt(interface);
        self.code.push(vt);
    }

    pub fn lifted_fun(&mut self, s: SymId) -> Rc<Lifted> {
        if let Some(l) = self.lifted.get(&s) {
            return l.clone();
        }
        let cx = self.cx;
        let free = self.an.of_fun(cx, s);
        let base = encode(cx.input.interner.get(cx.input.syms.sym(s).name));
        let name = self.fresh_name(&base);
        let params = self.param_types(s);
        let ret = self.return_type(s);
        let lifted = self.make_lifted(name, &free, params, ret);
        self.lifted.insert(s, lifted.clone());
        let f = cx.fun_of_sym[s.idx()];
        if f != u32::MAX {
            self.pending.push(Pending { lifted: lifted.clone(), body: PendingBody::Fun(FunId(f)) });
        }
        lifted
    }

    fn local_default(&mut self, s: SymId, i: usize, temps: &[(u16, JType)]) {
        let cx = self.cx;
        let f = cx.fun_of_sym[s.idx()];
        if f == u32::MAX {
            return self.unsupported("the default of a local def without a body");
        }
        let lifted = match self.lifted_defaults.get(&(s, i)) {
            Some(l) => l.clone(),
            None => {
                let free = self.an.of_fun(cx, s);
                let fun = self.lifted_fun(s);
                let name = format!("{}$default${}", fun.name, i + 1);
                let params: Vec<JType> = fun.params[..i].to_vec();
                let ret = fun.params[i].clone();
                let l = self.make_lifted(name, &free, params, ret);
                self.lifted_defaults.insert((s, i), l.clone());
                self.pending.push(Pending { lifted: l.clone(), body: PendingBody::Default(FunId(f), i) });
                l
            }
        };
        self.load_captures(&lifted);
        for (slot, t) in temps {
            self.load(*slot, t);
        }
        let owner = self.this_name.clone();
        let n = lifted.captures.len() + lifted.this as usize + lifted.params.len();
        self.invoke_desc(Invoke::Static, &owner, self.is_interface, &lifted.name, &lifted.desc, n, &lifted.ret);
    }

    // ---- local lazy vals ----

    fn lifted_lazy(&mut self, s: SymId) -> Rc<Lifted> {
        if let Some(l) = self.lifted.get(&s) {
            return l.clone();
        }
        let cx = self.cx;
        let free = self.an.of_lazy(cx, s);
        let base = encode(cx.input.interner.get(cx.input.syms.sym(s).name));
        let name = self.fresh_name(&format!("{}$lzy", base));
        let ret = self.sym_type(s);
        let cell = JType::L(Rc::from(OBJECT_REF));
        let lifted = self.make_lifted(name, &free, vec![cell], ret);
        self.lifted.insert(s, lifted.clone());
        if let Some(&init) = self.an.lazy_inits.get(&s) {
            self.pending.push(Pending { lifted: lifted.clone(), body: PendingBody::Lazy(s, init) });
        }
        lifted
    }

    fn load_lazy_cell(&mut self, s: SymId) {
        let cell = JType::L(Rc::from(OBJECT_REF));
        if let Some(l) = self.m.locals.get(&s).cloned() {
            self.load(l.slot, &cell);
        } else if let Some((field, _, _)) = self.captured_field(s) {
            self.load_this();
            let owner = self.this_name.clone();
            self.getfield(&owner, &field, &cell);
        } else {
            self.unsupported("a lazy val that is not in scope of the generated method");
            self.code.op(op::ACONST_NULL);
            self.code.push(VT::Null);
        }
    }

    fn lazy_read(&mut self, s: SymId) -> JType {
        let lifted = self.lifted_lazy(s);
        self.load_captures(&lifted);
        self.load_lazy_cell(s);
        let owner = self.this_name.clone();
        let n = lifted.captures.len() + lifted.this as usize + 1;
        self.invoke_desc(Invoke::Static, &owner, self.is_interface, &lifted.name, &lifted.desc, n, &lifted.ret);
        lifted.ret.clone()
    }

    // ---- blocks ----

    pub fn finish(&mut self, e: TExprId, mode: Mode) {
        match mode {
            Mode::Value(want) => self.expr(e, want),
            Mode::Tail => self.tail(e),
        }
    }

    /// Emits `e` in return position.
    pub fn tail(&mut self, e: TExprId) {
        let lines = (self.line_floor, self.line_hold);
        self.mark_line(e);
        self.tail_marked(e);
        (self.line_floor, self.line_hold) = lines;
    }

    fn tail_marked(&mut self, e: TExprId) {
        let cx = self.cx;
        let prog = cx.input.prog;
        // A widened value returned as a reference is boxed as one, not branch by branch.
        let boxed_whole = self.m.ret.is_ref() && self.widened_branches(e).is_some();
        match prog.expr(e) {
            TExpr::If(c, t, els @ Some(_)) if !boxed_whole => self.if_(c, t, els, Mode::Tail),
            TExpr::Match(scrut, cases) if !boxed_whole => self.match_(scrut, cases, Mode::Tail),
            TExpr::Block(stmts, res) if !boxed_whole => self.block(stmts, res, Mode::Tail),
            _ => {
                if let Some(tail) = &self.m.tail {
                    if let Some((recv, args)) = prog.self_call(tail.sym, tail.params.len(), e) {
                        return self.tail_jump(recv, args);
                    }
                }
                let ret = self.m.ret.clone();
                self.expr(e, &ret);
                self.return_value(&ret);
            }
        }
    }

    fn tail_jump(&mut self, recv: Option<TExprId>, args: ListRef) {
        let cx = self.cx;
        let tail = self.m.tail.as_ref().unwrap();
        let (sym, params, head) = (tail.sym, tail.params.clone(), tail.head);
        let types: Vec<JType> = params.iter().map(|p| self.m.locals[p].ty.clone()).collect();
        let items = cx.input.prog.expr_list(args).to_vec();
        let flags: Vec<bool> = cx.input.syms.sym(sym).sig.as_ref().map_or(Vec::new(), |sig| {
            sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.has_default)).collect()
        });
        let mark = self.code.locals_mark();
        let mut temps: Vec<(u16, JType)> = Vec::new();
        let is_default = |i: usize, a: TExprId| flags.get(i).copied().unwrap_or(false) && matches!(cx.input.prog.expr(a), TExpr::Unit);
        for (i, &a) in items.iter().enumerate() {
            if is_default(i, a) {
                self.adapt(&JType::V, &types[i]);
            } else {
                self.expr(a, &types[i]);
            }
            let slot = self.store_new(&types[i]);
            temps.push((slot, types[i].clone()));
        }
        for (i, &a) in items.iter().enumerate() {
            if !is_default(i, a) {
                continue;
            }
            if cx.input.syms.sym(sym).owner == Owner::Local {
                self.local_default(sym, i, &temps[..i]);
            } else {
                self.load_this();
                self.default_getter_call(sym, i, &temps[..i]);
            }
            let (slot, t) = temps[i].clone();
            self.store(slot, &t);
        }
        if let Some(r) = recv {
            let this_type = match self.this_class {
                Some(c) => self.class_type(c),
                None => JType::object(),
            };
            self.expr(r, &this_type);
            let slot = self.m.this_slot.unwrap_or(0);
            self.store(slot, &this_type);
        }
        for ((slot, t), p) in temps.iter().zip(&params) {
            self.load(*slot, t);
            let target = self.m.locals[p].slot;
            self.store(target, t);
        }
        self.code.locals_release(mark);
        self.goto(head);
    }

    pub fn block(&mut self, stmts: ListRef, res: TExprId, mode: Mode) {
        let mark = self.code.locals_mark();
        self.block_statements(stmts);
        self.finish(res, mode);
        if matches!(mode, Mode::Value(_)) {
            self.code.locals_release(mark);
        }
    }

    fn block_statements(&mut self, stmts: ListRef) {
        let cx = self.cx;
        let prog = cx.input.prog;
        for s in &prog.stmts[stmts.range()] {
            match *s {
                TStmt::Expr(x) => self.statement(x),
                TStmt::Val(sym, init) => self.local_val(sym, init),
                TStmt::Fun(f) => {
                    if cx.input.reach.funs[f.idx()] {
                        self.lifted_fun(prog.funs[f.idx()].sym);
                    }
                }
                TStmt::Pat(pat, init) => {
                    let t = self.expr_static(init);
                    let t = self.materialise_unit(t);
                    let slot = self.store_new(&t);
                    // The failure stands in front of the tests, so that the variables the
                    // pattern binds are the locals of the code that follows it.
                    let fail = self.code.new_label();
                    let start = self.code.new_label();
                    self.goto(start);
                    self.code.bind(fail);
                    self.match_error(slot, &t);
                    self.code.bind(start);
                    self.pattern(pat, slot, &t, fail);
                }
            }
        }
    }

    pub fn materialise_unit(&mut self, t: JType) -> JType {
        if t == JType::V {
            self.unit_value();
            JType::L(Rc::from(BOXED_UNIT))
        } else {
            t
        }
    }

    pub fn local_val(&mut self, sym: SymId, init: TExprId) {
        let cx = self.cx;
        let info = cx.input.syms.sym(sym);
        let cell_type = JType::L(Rc::from(OBJECT_REF));
        if info.mods & mods::LAZY != 0 {
            self.an.lazy_inits.insert(sym, init);
            self.new_object(OBJECT_REF);
            self.code.op(op::ACONST_NULL);
            self.code.push(VT::Null);
            self.invoke(Invoke::Special, OBJECT_REF, false, "<init>", &[JType::object()], &JType::V);
            let slot = self.store_new(&cell_type);
            let ty = self.sym_type(sym);
            self.m.locals.insert(sym, Local { slot, ty, cell: true });
            return;
        }
        let ty = self.sym_type(sym);
        if self.m.cells.contains(&sym) {
            // The cell exists before the initialiser runs, which may refer to the val itself.
            self.new_object(OBJECT_REF);
            self.code.op(op::ACONST_NULL);
            self.code.push(VT::Null);
            self.invoke(Invoke::Special, OBJECT_REF, false, "<init>", &[JType::object()], &JType::V);
            let slot = self.store_new(&cell_type);
            self.m.locals.insert(sym, Local { slot, ty: ty.clone(), cell: true });
            self.load(slot, &cell_type);
            let object = JType::object();
            self.expr(init, &ty);
            self.adapt(&ty, &object);
            self.putfield(OBJECT_REF, "elem", &object);
            return;
        }
        self.expr(init, &ty);
        let slot = self.store_new(&ty);
        self.m.locals.insert(sym, Local { slot, ty, cell: false });
    }

    // ---- conditions ----

    fn if_(&mut self, c: TExprId, t: TExprId, els: Option<TExprId>, mode: Mode) {
        let otherwise = self.code.new_label();
        self.cond(c, otherwise, false);
        let mark = self.code.locals_mark();
        match (els, mode) {
            (None, Mode::Value(want)) => {
                self.statement(t);
                self.code.locals_release(mark);
                if *want == JType::V {
                    self.code.bind(otherwise);
                } else {
                    let done = self.code.new_label();
                    self.adapt(&JType::V, want);
                    self.goto(done);
                    self.code.pop();
                    self.code.bind(otherwise);
                    self.adapt(&JType::V, want);
                    self.code.bind(done);
                }
            }
            (None, Mode::Tail) => {
                self.statement(t);
                self.code.locals_release(mark);
                self.code.bind(otherwise);
                let ret = self.m.ret.clone();
                self.adapt(&JType::V, &ret);
                self.return_value(&ret);
            }
            (Some(els), Mode::Tail) => {
                self.tail(t);
                self.code.locals_release(mark);
                self.code.bind(otherwise);
                self.tail(els);
                self.code.locals_release(mark);
            }
            (Some(els), Mode::Value(want)) => {
                let done = self.code.new_label();
                self.expr(t, want);
                self.code.locals_release(mark);
                self.goto(done);
                if *want != JType::V {
                    self.code.pop();
                }
                self.code.bind(otherwise);
                self.expr(els, want);
                self.code.locals_release(mark);
                self.code.bind(done);
            }
        }
    }

    /// A boolean expression as a value.
    fn cond_value(&mut self, e: TExprId) {
        let no = self.code.new_label();
        let done = self.code.new_label();
        self.cond(e, no, false);
        self.iconst(1);
        self.goto(done);
        self.code.pop();
        self.code.bind(no);
        self.iconst(0);
        self.code.bind(done);
    }

    /// Jumps to `target` when `e` is `jump_if`, falls through otherwise.
    pub fn cond(&mut self, e: TExprId, target: Label, jump_if: bool) {
        let prog = self.cx.input.prog;
        match prog.expr(e) {
            TExpr::Bool(v) => {
                if v == jump_if {
                    self.goto(target);
                }
            }
            TExpr::Unary(UnOp::BoolNot, a) => self.cond(a, target, !jump_if),
            TExpr::Prim(PrimOp::BoolAnd, a, b) => {
                if jump_if {
                    let skip = self.code.new_label();
                    self.cond(a, skip, false);
                    self.cond(b, target, true);
                    self.code.bind(skip);
                } else {
                    self.cond(a, target, false);
                    self.cond(b, target, false);
                }
            }
            TExpr::Prim(PrimOp::BoolOr, a, b) => {
                if jump_if {
                    self.cond(a, target, true);
                    self.cond(b, target, true);
                } else {
                    let skip = self.code.new_label();
                    self.cond(a, skip, true);
                    self.cond(b, target, false);
                    self.code.bind(skip);
                }
            }
            TExpr::Prim(op @ (PrimOp::Lt | PrimOp::Le | PrimOp::Gt | PrimOp::Ge | PrimOp::RefEq | PrimOp::RefNe), a, b) => {
                self.compare(op, a, b, target, jump_if)
            }
            TExpr::TypeTest(inner, test) => {
                // No cast to the static type: `box.fruit.isInstanceOf[Apple]` asks because the
                // static type may be wrong.
                let boxed = JType::object();
                self.expr(inner, &boxed);
                let mark = self.code.locals_mark();
                let slot = self.store_new(&boxed);
                self.test(test, slot, &boxed, None, target, jump_if);
                self.code.locals_release(mark);
            }
            _ => {
                self.expr(e, &JType::Z);
                self.jump_if(if jump_if { op::IFNE } else { op::IFEQ }, 1, target);
            }
        }
    }

    fn compare(&mut self, op_: PrimOp, a: TExprId, b: TExprId, target: Label, jump_if: bool) {
        let (ta, tb) = (self.static_type(a), self.static_type(b));
        let is_eq = matches!(op_, PrimOp::RefEq | PrimOp::RefNe);
        // A NaN operand makes the comparison false: the jump that tests it is not taken, the
        // one that tests its negation is, so NaN's side of `dcmpg` and `dcmpl` follows the
        // comparison as written.
        let nan_high = matches!(op_, PrimOp::Lt | PrimOp::Le);
        // The condition under which the jump is taken.
        let op_ = match (op_, jump_if) {
            (o, true) => o,
            (PrimOp::Lt, false) => PrimOp::Ge,
            (PrimOp::Le, false) => PrimOp::Gt,
            (PrimOp::Gt, false) => PrimOp::Le,
            (PrimOp::Ge, false) => PrimOp::Lt,
            (PrimOp::RefEq, false) => PrimOp::RefNe,
            (_, false) => PrimOp::RefEq,
        };
        let kind = match (ta.numeric_rank(), tb.numeric_rank()) {
            (Some(x), Some(y)) => Some(rank_type(x.max(y))),
            (Some(x), None) if tb.is_ref() && !is_eq => Some(rank_type(x)),
            (None, Some(y)) if ta.is_ref() && !is_eq => Some(rank_type(y)),
            _ => None,
        };
        if let Some(k) = kind {
            self.expr(a, &k);
            self.expr(b, &k);
            match k {
                JType::I => {
                    let insn = match op_ {
                        PrimOp::Lt => op::IF_ICMPLT,
                        PrimOp::Le => op::IF_ICMPLE,
                        PrimOp::Gt => op::IF_ICMPGT,
                        PrimOp::Ge => op::IF_ICMPGE,
                        PrimOp::RefEq => op::IF_ICMPEQ,
                        _ => op::IF_ICMPNE,
                    };
                    self.jump_if(insn, 2, target);
                }
                _ => {
                    let cmp = match (&k, nan_high) {
                        (JType::J, _) => op::LCMP,
                        (JType::F, true) => op::FCMPG,
                        (JType::F, false) => op::FCMPL,
                        (_, true) => op::DCMPG,
                        (_, false) => op::DCMPL,
                    };
                    self.code.op(cmp);
                    self.code.popn(2);
                    self.code.push(VT::Int);
                    let insn = match op_ {
                        PrimOp::Lt => op::IFLT,
                        PrimOp::Le => op::IFLE,
                        PrimOp::Gt => op::IFGT,
                        PrimOp::Ge => op::IFGE,
                        PrimOp::RefEq => op::IFEQ,
                        _ => op::IFNE,
                    };
                    self.jump_if(insn, 1, target);
                }
            }
            return;
        }
        if !is_eq {
            // Strings and other comparables: `compareTo`.
            let comparable = JType::L(Rc::from("java/lang/Comparable"));
            self.expr(a, &comparable);
            let object = JType::object();
            self.expr(b, &object);
            self.invoke(Invoke::Interface, "java/lang/Comparable", true, "compareTo", &[object], &JType::I);
            let insn = match op_ {
                PrimOp::Lt => op::IFLT,
                PrimOp::Le => op::IFLE,
                PrimOp::Gt => op::IFGT,
                _ => op::IFGE,
            };
            return self.jump_if(insn, 1, target);
        }
        if ta == JType::Z && tb == JType::Z {
            self.expr(a, &JType::Z);
            self.expr(b, &JType::Z);
            return self.jump_if(if op_ == PrimOp::RefEq { op::IF_ICMPEQ } else { op::IF_ICMPNE }, 2, target);
        }
        if ta == JType::V && tb == JType::V {
            self.statement(a);
            self.statement(b);
            if op_ == PrimOp::RefEq {
                self.iconst(1);
                self.jump_if(op::IFNE, 1, target);
            }
            return;
        }
        let object = JType::object();
        let by_value = ta.is_string() || tb.is_string() || !ta.is_ref() || !tb.is_ref();
        self.expr(a, &object);
        self.expr(b, &object);
        if by_value {
            // Strings and boxed primitives compare by value, as `===` does on the JS side.
            self.invoke(Invoke::Static, "java/util/Objects", false, "equals", &[object.clone(), object], &JType::Z);
            self.jump_if(if op_ == PrimOp::RefEq { op::IFNE } else { op::IFEQ }, 1, target);
        } else {
            self.jump_if(if op_ == PrimOp::RefEq { op::IF_ACMPEQ } else { op::IF_ACMPNE }, 2, target);
        }
    }

    // ---- primitive operations ----

    fn prim(&mut self, op_: PrimOp, a: TExprId, b: TExprId) -> JType {
        use PrimOp::*;
        let (t, insn) = match op_ {
            IntAdd => (JType::I, op::IADD),
            IntSub => (JType::I, op::ISUB),
            IntMul => (JType::I, op::IMUL),
            IntDiv => (JType::I, op::IDIV),
            IntRem => (JType::I, op::IREM),
            IntAnd => (JType::I, op::IAND),
            IntOr => (JType::I, op::IOR),
            IntXor => (JType::I, op::IXOR),
            IntShl => (JType::I, op::ISHL),
            IntShr => (JType::I, op::ISHR),
            IntUshr => (JType::I, op::IUSHR),
            LongAdd => (JType::J, op::LADD),
            LongSub => (JType::J, op::LSUB),
            LongMul => (JType::J, op::LMUL),
            LongDiv => (JType::J, op::LDIV),
            LongRem => (JType::J, op::LREM),
            LongAnd => (JType::J, op::LAND),
            LongOr => (JType::J, op::LOR),
            LongXor => (JType::J, op::LXOR),
            LongShl => (JType::J, op::LSHL),
            LongShr => (JType::J, op::LSHR),
            LongUshr => (JType::J, op::LUSHR),
            DoubleAdd => (JType::D, op::DADD),
            DoubleSub => (JType::D, op::DSUB),
            DoubleMul => (JType::D, op::DMUL),
            DoubleDiv => (JType::D, op::DDIV),
            DoubleRem => (JType::D, op::DREM),
            FloatAdd => (JType::F, op::FADD),
            FloatSub => (JType::F, op::FSUB),
            FloatMul => (JType::F, op::FMUL),
            FloatDiv => (JType::F, op::FDIV),
            FloatRem => (JType::F, op::FREM),
            BoolStrictAnd => (JType::Z, op::IAND),
            BoolStrictOr => (JType::Z, op::IOR),
            BoolXor => {
                self.expr(a, &JType::Z);
                self.expr(b, &JType::Z);
                self.code.op(op::IXOR);
                self.code.pop();
                return JType::Z;
            }
            Eq | Ne => {
                let object = JType::object();
                self.expr(a, &object);
                self.expr(b, &object);
                self.helper_call("equal", 2);
                if op_ == Ne {
                    self.iconst(1);
                    self.code.op(op::IXOR);
                    self.code.pop();
                }
                return JType::Z;
            }
            Lt | Le | Gt | Ge | RefEq | RefNe | BoolAnd | BoolOr => return self.cond_of_prim(op_, a, b),
        };
        self.expr(a, &t);
        // The shift distance of a long is an int.
        let shift = matches!(op_, LongShl | LongShr | LongUshr);
        if shift {
            let tb = self.static_type(b);
            self.expr(b, &tb);
            self.adapt(&tb, &JType::I);
        } else {
            self.expr(b, &t);
        }
        self.code.op(insn);
        self.code.pop();
        t
    }

    fn cond_of_prim(&mut self, op_: PrimOp, a: TExprId, b: TExprId) -> JType {
        let no = self.code.new_label();
        let done = self.code.new_label();
        match op_ {
            PrimOp::BoolAnd => {
                self.cond(a, no, false);
                self.cond(b, no, false);
            }
            PrimOp::BoolOr => {
                let yes = self.code.new_label();
                self.cond(a, yes, true);
                self.cond(b, no, false);
                self.code.bind(yes);
            }
            _ => self.compare(op_, a, b, no, false),
        }
        self.iconst(1);
        self.goto(done);
        self.code.pop();
        self.code.bind(no);
        self.iconst(0);
        self.code.bind(done);
        JType::Z
    }

    fn unary(&mut self, op_: UnOp, a: TExprId) -> JType {
        use UnOp::*;
        // A literal widened to a double is the double constant, as scalac types the literal.
        if let (IntToDouble, TExpr::Int(v)) = (op_, self.cx.input.prog.expr(a)) {
            self.dconst(v as f64);
            return JType::D;
        }
        let (from, to, insns): (JType, JType, &[u8]) = match op_ {
            IntNeg => (JType::I, JType::I, &[op::INEG]),
            LongNeg => (JType::J, JType::J, &[op::LNEG]),
            DoubleNeg => (JType::D, JType::D, &[op::DNEG]),
            FloatNeg => (JType::F, JType::F, &[op::FNEG]),
            IntToLong => (JType::I, JType::J, &[op::I2L]),
            LongToDouble => (JType::J, JType::D, &[op::L2D]),
            LongToInt => (JType::J, JType::I, &[op::L2I]),
            DoubleToInt => (JType::D, JType::I, &[op::D2I]),
            DoubleToLong => (JType::D, JType::J, &[op::D2L]),
            CharToInt => (JType::C, JType::I, &[]),
            CharToLong => (JType::C, JType::J, &[op::I2L]),
            IntToChar => (JType::I, JType::C, &[op::I2C]),
            IntToByte => (JType::I, JType::B, &[op::I2B]),
            IntToShort => (JType::I, JType::S, &[op::I2S]),
            IntToFloat => (JType::I, JType::F, &[op::I2F]),
            IntToDouble => (JType::I, JType::D, &[op::I2D]),
            ByteToShort => (JType::B, JType::S, &[]),
            ByteToInt => (JType::B, JType::I, &[]),
            ShortToInt => (JType::S, JType::I, &[]),
            LongToFloat => (JType::J, JType::F, &[op::L2F]),
            FloatToInt => (JType::F, JType::I, &[op::F2I]),
            FloatToLong => (JType::F, JType::J, &[op::F2L]),
            FloatToDouble => (JType::F, JType::D, &[op::F2D]),
            DoubleToFloat => (JType::D, JType::F, &[op::D2F]),
            BoolNot => {
                self.expr(a, &JType::Z);
                self.iconst(1);
                self.code.op(op::IXOR);
                self.code.pop();
                return JType::Z;
            }
            IntNot => {
                self.expr(a, &JType::I);
                self.iconst(-1);
                self.code.op(op::IXOR);
                self.code.pop();
                return JType::I;
            }
            LongNot => {
                self.expr(a, &JType::J);
                self.lconst(-1);
                self.code.op(op::LXOR);
                self.code.pop();
                return JType::J;
            }
        };
        self.expr(a, &from);
        for &i in insns {
            self.code.op(i);
        }
        self.code.pop();
        let vt = self.vt(&to);
        self.code.push(vt);
        to
    }

    /// Calls a definition of `std/jvm.scala`; the arguments are on the stack.
    pub fn helper_call(&mut self, name: &str, n_args: usize) -> JType {
        let Some(&s) = self.cx.helpers.defs.get(name) else {
            self.unsupported(&format!("the runtime helper {} is missing", name));
            self.code.popn(n_args);
            return JType::V;
        };
        let m = self.mref(s);
        // The module goes under the arguments, which are already evaluated.
        let mark = self.code.locals_mark();
        let mut temps = Vec::new();
        for t in m.params.iter().rev() {
            temps.push((self.store_new(t), t.clone()));
        }
        let owner_type = JType::L(m.owner.clone());
        self.getstatic(&m.owner, "MODULE$", &owner_type);
        for (slot, t) in temps.iter().rev() {
            self.load(*slot, t);
        }
        self.code.locals_release(mark);
        self.invoke_mref(&m);
        m.ret.clone()
    }

    pub fn match_error(&mut self, slot: u16, t: &JType) {
        self.new_object(MATCH_ERROR);
        let object = JType::object();
        self.load_slot_as(slot, t, &object);
        self.invoke(Invoke::Special, MATCH_ERROR, false, "<init>", &[object], &JType::V);
        self.code.op(op::ATHROW);
        self.code.pop();
        self.code.end_path();
    }
}

/// The stack type of a comparison or operation between operands of the given ranks.
fn rank_type(rank: u8) -> JType {
    use crate::typer::prims::*;
    match rank {
        R_LONG => JType::J,
        R_FLOAT => JType::F,
        R_DOUBLE => JType::D,
        _ => JType::I,
    }
}

fn prim_type(op: PrimOp) -> JType {
    use PrimOp::*;
    match op {
        IntAdd | IntSub | IntMul | IntDiv | IntRem | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr => JType::I,
        LongAdd | LongSub | LongMul | LongDiv | LongRem | LongAnd | LongOr | LongXor | LongShl | LongShr | LongUshr => JType::J,
        DoubleAdd | DoubleSub | DoubleMul | DoubleDiv | DoubleRem => JType::D,
        FloatAdd | FloatSub | FloatMul | FloatDiv | FloatRem => JType::F,
        _ => JType::Z,
    }
}

fn unary_type(op: UnOp) -> JType {
    use UnOp::*;
    match op {
        IntNeg | IntNot | LongToInt | DoubleToInt | CharToInt | FloatToInt | ByteToInt | ShortToInt => JType::I,
        LongNeg | LongNot | IntToLong | DoubleToLong | CharToLong | FloatToLong => JType::J,
        DoubleNeg | LongToDouble | FloatToDouble | IntToDouble => JType::D,
        FloatNeg | IntToFloat | LongToFloat | DoubleToFloat => JType::F,
        BoolNot => JType::Z,
        IntToChar => JType::C,
        ByteToShort => JType::S,
        IntToByte => JType::B,
        IntToShort => JType::S,
    }
}

/// The method of a class file that a call of `name` with `arity` parameters and teq's erased
/// descriptor `own` resolves to: the exact descriptor, else the one with the most parameters
/// alike, the first of them on a tie; never a static method or a bridge.
/// The bound of a higher-kinded parameter applied to its arguments (`Gen::applied_upper`): a type
/// that erases as the application does, or the array of an argument.
enum Applied {
    Is(TypeId),
    ArrayOf(TypeId),
}

/// What a part of a union erases to, as scalac's `erasedLub` reads it: a class whose bases it
/// walks, a value class, an array of such a part, the bottom types, or an erasure of another kind.
#[derive(Clone, PartialEq)]
enum Lub {
    Class(ClassId),
    /// A value class and its erasure, its underlying value's.
    Value(ClassId, JType),
    Array(Box<Lub>),
    Nothing,
    Null,
    Erased(JType),
}

/// scalac's `specialErasure`: no tuple class at run time extends the tuple supertypes, which
/// erase to what every tuple is, and `Singleton` is a reference of any class.
fn special_erasure(cx: &Cx, c: ClassId) -> Option<&'static str> {
    match scala_class(cx, c)? {
        "Tuple" | "NonEmptyTuple" => Some(PRODUCT),
        "Singleton" => Some(OBJECT),
        _ => None,
    }
}

/// The elements of a tuple class, `Tuple2`'s two.
fn tuple_arity(cx: &Cx, c: ClassId) -> Option<usize> {
    scala_class(cx, c)?.strip_prefix("Tuple")?.parse().ok()
}

/// The name of a top-level class of the package `scala`, which scalac's erasure treats by name.
fn scala_class<'c>(cx: &'c Cx, c: ClassId) -> Option<&'c str> {
    let info = cx.input.syms.class(c);
    let Owner::Package(p) = info.owner else { return None };
    let pkg = cx.input.syms.pkg(p);
    let top_level = pkg.parent.map_or(false, |r| cx.input.syms.pkg(r).parent.is_none());
    if info.kind == ClassKind::Object || !top_level || cx.input.interner.get(pkg.name) != "scala" {
        return None;
    }
    Some(cx.input.interner.get(info.name))
}

fn pick_method<'c>(cf: &'c crate::classfile::ClassFile, name: &str, arity: usize, own: &str) -> Option<&'c crate::classfile::Method> {
    let (own_params, own_ret) = parse_method_desc(own);
    let mut best: Option<(&crate::classfile::Method, usize)> = None;
    for m in cf.methods.iter().filter(|m| m.name == name && m.access & (ACC_STATIC | ACC_BRIDGE) == 0) {
        if m.descriptor == own {
            return Some(m);
        }
        let (params, ret) = parse_method_desc(&m.descriptor);
        if params.len() != arity {
            continue;
        }
        let score = 2 * params.iter().zip(&own_params).filter(|(a, b)| a == b).count() + (ret == own_ret) as usize;
        if best.map_or(true, |(_, b)| score > b) {
            best = Some((m, score));
        }
    }
    best.map(|(m, _)| m)
}

/// A primitive's class in scalac's names, which orders two primitives in an intersection.
fn scala_name_of(t: &JType) -> &'static str {
    match t {
        JType::Z => "scala.Boolean",
        JType::B => "scala.Byte",
        JType::C => "scala.Char",
        JType::S => "scala.Short",
        JType::I => "scala.Int",
        JType::J => "scala.Long",
        JType::F => "scala.Float",
        JType::D => "scala.Double",
        _ => "scala.Unit",
    }
}

/// The type an element descriptor of an array names.
fn desc_type(d: &str) -> JType {
    match d.as_bytes().first() {
        Some(b'Z') => JType::Z,
        Some(b'B') => JType::B,
        Some(b'C') => JType::C,
        Some(b'S') => JType::S,
        Some(b'I') => JType::I,
        Some(b'J') => JType::J,
        Some(b'F') => JType::F,
        Some(b'D') => JType::D,
        Some(b'L') => JType::L(Rc::from(d[1..].trim_end_matches(';'))),
        _ => JType::L(Rc::from(d)),
    }
}
