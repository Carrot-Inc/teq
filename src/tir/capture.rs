//! The typed form of the bodies that the pickle of their right-hand sides needs and the
//! executable IR does not keep: what the typer resolved, kept
//! beside the IR as it lowers it, in scalac's form after `PostTyper`. The product modes set it
//! (`Program::capture`); off them nothing is recorded, and on them the executable IR is the
//! same.
//!
//! A record describes the node it is keyed by, relative to that node's children, and names no
//! other expression; a copy of the node takes its records as they are. The one exception is an
//! inline call's record, which holds the receiver and the arguments as they were typed at the
//! call, before the expansion that stands for the call in the IR.

use crate::intern::{FxMap, Name};
use crate::source::FileId;
use crate::tir::{TExprId, TPatId};
use crate::types::{ClassId, SymId, TList, TypeId};

/// How many places of the records a copy lost the census gives.
pub const LOST_PLACES: usize = 5;

#[derive(Default)]
pub struct Capture {
    /// The type arguments of a call of a method or a constructor with type parameters (the
    /// method's own, a constructor's the class's), which the call nodes leave out.
    pub targs: FxMap<TExprId, TList>,
    /// What a node the typer lowered stands for.
    pub forms: FxMap<TExprId, Form>,
    /// The source's layers around a node that the typer handed back as the node itself: an
    /// ascription, a cast, an `@unchecked`, a sequence passed as repeated arguments, a name.
    /// Innermost first.
    pub wraps: FxMap<TExprId, Vec<Wrap>>,
    pub pats: FxMap<TPatId, PatForm>,
    /// What a local symbol lost: modifiers, the type of a temporary typed `Any`.
    pub locals: FxMap<SymId, Local>,
    /// The calls the typer replaced by what they compute (a tuple's builtin member, a lambda
    /// applied), by the node it made of each.
    pub builtin_calls: FxMap<TExprId, BuiltinCall>,
    /// The receiver a node reached its member through and leaves out, by the node: the
    /// `Quotes` of an extension the typer made a template call of (`x.show`), the `q.reflect`
    /// of a member of the reflection API (`q.reflect.TypeRepr`), which scalac's pickle selects
    /// through. Like an inline call's record, it holds an expression.
    pub receivers: FxMap<TExprId, TExprId>,
    /// The evidence a call of a `@jvmEvidence` member passes where the member's signature leaves
    /// it out (JavaScript, the interpreter): what the typer selected for each tag, in the tags'
    /// order, which the executable call does not pass and scalac's pickle passes in the member's
    /// last clause. Like an inline call's record, it holds expressions.
    pub evidence: FxMap<TExprId, Vec<TExprId>>,
    /// The tree of a pickle a node was typed from, where a library's or an upstream product's
    /// body converted from TASTy holds it (its TASTy file of the loader and address): the
    /// place the pickle gives it, as scalac's pickle keeps an inlined body's.
    pub trees: FxMap<TExprId, (u32, u32)>,
    /// The classes an expansion of an ordinary inline method made (an anonymous class of its
    /// body), which the pickle holds none of: it holds the call.
    pub expansion_classes: FxMap<ClassId, ()>,
    /// The inline calls an expansion stands for, by the expression the expansion gave; nested
    /// expansions that gave one expression, innermost first.
    pub inline_calls: FxMap<TExprId, Vec<InlineCall>>,
    /// The statements of a block its typed form holds none of, by the block's file and source
    /// offset, each with the statement it stands before: its imports, what the search of an
    /// inline expansion or a macro there sees, which a regenerated body takes from the pickle,
    /// and its local inline methods. Keyed by the source, an expansion's copy of a block reads
    /// its definition's, as scalac's expansion does. Each record has its statement's source
    /// start beside: consecutive imports stand before one statement, and a statement typed again
    /// replaces its own records alone.
    pub block_stmts: FxMap<(FileId, u32), Vec<(u32, u32, BlockStmt)>>,
    /// The definitions' annotations as typed, by the file and the source's `new` of each
    /// (`ast::Annot::instance`): one tree for every symbol the annotation lands on, as scalac's
    /// pickle shares it. One that does not type has none.
    pub annotations: FxMap<(FileId, crate::ast::ExprId), TExprId>,
    /// The keys of the records each file's typing made, which a retype of the file drops.
    pub made: FxMap<FileId, Vec<Key>>,
    /// What each file's typing changed of records made before (a value replaced, a record
    /// deleted), each with how many records the file had made then: the journal that dropping
    /// an attempt undoes, the changes and the records made after them newest first. Kept while
    /// the files are typed.
    pub changed: FxMap<FileId, Vec<(u32, Prior)>>,
    /// The copies a worker made of a node another worker made (`Program::typed`), whose records
    /// are that worker's until the merge: each copy takes them there, as the file's that made it.
    pub pending: Vec<(TExprId, TExprId, FileId)>,
    /// How many records the compactions dropped: of typings no body holds.
    pub discarded: u64,
    /// Whether a file's bodies are captured, by file: the owned sources' (the product mode's
    /// program files, none of the std's, of a jar or of a library body).
    pub owned: Vec<bool>,
    /// Whether a file's definitions' annotations are typed, by file: every program source's,
    /// owned or not, since an owned class's API reads its inherited members' (`--own`).
    pub annotated: Vec<bool>,
    /// The quote copier's check of its copies (`Worker::capture_check_copy`): the records of
    /// the copied nodes, patterns and binders found on their copies, those missing from them,
    /// and a few places of those.
    pub copies_checked: u64,
    pub copies_lost: u64,
    pub lost_at: Vec<String>,
    /// The records of the quotes' bodies, which every worker's copier reads, one table for all
    /// the workers, and whether this capture published any: what drops or puts back a record
    /// of its own then does it there too.
    pub published: std::sync::Arc<std::sync::RwLock<Published>>,
    pub publishing: bool,
}

/// The records of the quotes' bodies, published by the worker that typed each body once it is
/// typed (`Worker::capture_publish`), before any other worker can read the body: a quote typed
/// by one worker, or before the workers forked, runs in another's macro expansion, whose copy of
/// the body takes its records from here. A record is not changed once published; the merge
/// renumbers the table with the capture, and a retype of a file drops the file's.
#[derive(Default)]
pub struct Published {
    pub targs: FxMap<TExprId, TList>,
    pub forms: FxMap<TExprId, Form>,
    pub wraps: FxMap<TExprId, Vec<Wrap>>,
    pub pats: FxMap<TPatId, PatForm>,
    pub locals: FxMap<SymId, Local>,
    pub inline_calls: FxMap<TExprId, Vec<InlineCall>>,
    pub builtin_calls: FxMap<TExprId, BuiltinCall>,
    pub receivers: FxMap<TExprId, TExprId>,
    pub evidence: FxMap<TExprId, Vec<TExprId>>,
    pub trees: FxMap<TExprId, (u32, u32)>,
}

/// The published `key` as `own` has it: dropped where it has none.
fn sync<K: Copy + Eq + std::hash::Hash, V>(table: &mut FxMap<K, V>, key: K, own: Option<V>) {
    match own {
        Some(v) => {
            table.insert(key, v);
        }
        None => {
            table.remove(&key);
        }
    }
}

impl Prior {
    /// The record the prior value is of.
    pub fn key(&self) -> Key {
        match *self {
            Prior::Targs(e, _) => Key::Targs(e),
            Prior::Form(e, _) => Key::Form(e),
            Prior::Wraps(e, _) => Key::Wrap(e),
            Prior::Inline(e, _) => Key::Inline(e),
            Prior::Builtin(e, _) => Key::Builtin(e),
            Prior::Receiver(e, _) => Key::Receiver(e),
            Prior::Evidence(e, _) => Key::Evidence(e),
            Prior::Tree(e, _) => Key::Tree(e),
            Prior::Pat(p, _) => Key::Pat(p),
            Prior::Local(s, _) => Key::Local(s),
        }
    }
}

impl Published {
    fn has(&self, k: Key) -> bool {
        match k {
            Key::Targs(e) => self.targs.contains_key(&e),
            Key::Form(e) => self.forms.contains_key(&e),
            Key::Wrap(e) => self.wraps.contains_key(&e),
            Key::Inline(e) => self.inline_calls.contains_key(&e),
            Key::Builtin(e) => self.builtin_calls.contains_key(&e),
            Key::Receiver(e) => self.receivers.contains_key(&e),
            Key::Evidence(e) => self.evidence.contains_key(&e),
            Key::Tree(e) => self.trees.contains_key(&e),
            Key::Pat(p) => self.pats.contains_key(&p),
            Key::Local(s) => self.locals.contains_key(&s),
            Key::Class(_) => false,
        }
    }

    fn held(&self) -> usize {
        use crate::held::{array, table};
        table(&self.targs)
            + table(&self.forms)
            + table(&self.wraps)
            + self.wraps.values().map(array).sum::<usize>()
            + table(&self.pats)
            + table(&self.locals)
            + table(&self.inline_calls)
            + self.inline_calls.values().map(|c| array(c) + c.iter().map(|c| array(&c.args)).sum::<usize>()).sum::<usize>()
            + table(&self.builtin_calls)
            + self.builtin_calls.values().map(|c| array(&c.args)).sum::<usize>()
            + table(&self.receivers)
            + table(&self.evidence)
            + self.evidence.values().map(array).sum::<usize>()
            + table(&self.trees)
    }
}

/// The records of one node, as a copy of it takes them.
#[derive(Default, Clone, Debug)]
pub struct NodeRecords {
    pub targs: Option<TList>,
    pub form: Option<Form>,
    pub wraps: Vec<Wrap>,
    pub inline_calls: Vec<InlineCall>,
    pub builtin: Option<BuiltinCall>,
    pub receiver: Option<TExprId>,
    pub evidence: Vec<TExprId>,
    pub tree: Option<(u32, u32)>,
}

impl NodeRecords {
    pub fn is_empty(&self) -> bool {
        self.targs.is_none() && self.form.is_none() && self.wraps.is_empty() && self.inline_calls.is_empty() && self.builtin.is_none() && self.receiver.is_none() && self.evidence.is_empty() && self.tree.is_none()
    }
}

/// A record's key, in `Capture::made`: its table and what it is keyed by, so that dropping it
/// leaves the node's records in the other tables, which another typing may have made.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Targs(TExprId),
    Form(TExprId),
    Wrap(TExprId),
    Inline(TExprId),
    Builtin(TExprId),
    Receiver(TExprId),
    Evidence(TExprId),
    Tree(TExprId),
    Pat(TPatId),
    Local(SymId),
    Class(ClassId),
}

impl Key {
    /// The expression the record is keyed by, where it is one.
    pub fn expr(self) -> Option<TExprId> {
        match self {
            Key::Targs(e) | Key::Form(e) | Key::Wrap(e) | Key::Inline(e) | Key::Builtin(e) | Key::Receiver(e) | Key::Evidence(e) | Key::Tree(e) => Some(e),
            Key::Pat(_) | Key::Local(_) | Key::Class(_) => None,
        }
    }
}

/// A record as it was before a typing changed it (`Capture::changed`).
#[derive(Clone, Debug)]
pub enum Prior {
    Targs(TExprId, TList),
    Form(TExprId, Form),
    Wraps(TExprId, Vec<Wrap>),
    Inline(TExprId, Vec<InlineCall>),
    Builtin(TExprId, BuiltinCall),
    Receiver(TExprId, TExprId),
    Evidence(TExprId, Vec<TExprId>),
    Tree(TExprId, (u32, u32)),
    Pat(TPatId, PatForm),
    Local(SymId, Local),
}

/// A statement of a block that its typed form leaves out (`Capture::block_stmts`).
#[derive(Clone, Copy)]
pub enum BlockStmt {
    Import(crate::typer::ResolvedImport),
    /// A local inline method, its body the record the definition check stored.
    InlineDef(SymId),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    /// A `Lambda` without parameters that passes a by-name argument: the argument itself.
    ByName,
    /// A literal the typer folded the operator of its operand to (`!true` as `false`): the
    /// operator applied to the literal it inverts.
    Folded(Name),
    /// A literal that stands for a read of a constant, an `inline val`'s or a constant member's
    /// behind `this` or an object: the reference, which scalac's pickle keeps (`k * 2`).
    Constant(SymId),
    /// A `Unit` that stands for an omitted argument: the application of the parameter's
    /// default getter to the arguments of the clauses before it.
    Default,
    /// A `SeqLit` of repeated arguments: `REPEATED` with the elements' type.
    Repeated(TypeId),
    /// A `Unary` the typer put around an operand of a builtin operator to promote it: the
    /// operand, as scalac selects the operator's overload for the operand's own type.
    Promotion,
    /// A `Unary` the typer put around a value adapted to a wider numeric type: scalac's
    /// `Int.int2long(i)`, the companion's conversion named by the two types.
    Widening,
    /// A node that is `e.asInstanceOf[T]` of its child (a numeric conversion, an unboxing, the
    /// zero of `null.asInstanceOf[Int]`).
    Cast(TypeId),
    /// A `TypeTest`: `e.isInstanceOf[T]` with the type as written, arguments and refinements
    /// included.
    Test(TypeId),
    /// A builtin operator or member of `Any` whose name the node does not tell (`eq` from
    /// `==`, `equals`, `hashCode`, `getClass`, `synchronized` as the block of its receiver and
    /// its body): `recv.name(args)`, the receiver the first child.
    Op(Name),
    /// `Op` whose operands the node holds in the other order: the right one evaluates first
    /// where the left one's evaluation is quiet, which scalac's backend does.
    SwappedOp(Name),
    /// `super.name(args)` of a member of `Any` that the typer wrote over `this`, its first child.
    SuperOp(Name),
    /// A member the typer synthesizes for an enum's companion (`values`, `valueOf`,
    /// `fromOrdinal`): `E.name(args)` of the enum of the node's type, the arguments the
    /// children after the first, teq's array of the values.
    EnumMember(Name),
    /// A `StrConcat` of an interpolation: `StringContext(parts*).kind(args*)`, `parts` the
    /// literal type of each part as written (`Scanners.getStringPart`), the concatenation
    /// holding it processed (an `s` interpolation's escapes) or leaving it out (an empty one).
    Interp { kind: Name, parts: TList },
    /// A string literal of the parts of a `StringContext` (of a pattern, of an interpolator of a
    /// library) that holds a part otherwise than it is written: the literal type of the part as
    /// written.
    Written(TypeId),
    /// An `ArrayLit` that packs a comprehension's variables: `TupleN.apply[..](vars)` of their
    /// types.
    Pack(TList),
    /// A `Block` whose statements read a packed comprehension's variables through `Index`:
    /// `(x$1: @unchecked) match { case TupleN(vars) => value }` of their types.
    Unpack(TList),
    /// A node that calls the member in the place of a call of it: a `Js` template's, a
    /// JavaScript node's (`JsImport`, `JsGlobal`, `JsSelect`), a std helper's that takes the
    /// receiver as its first argument (`Product`'s members).
    Member(SymId),
    /// A `Return` from the method.
    Return(SymId),
    /// A `New` of a given's class that stands for a call of the given.
    GivenCall(SymId),
    /// A member of a `js.Dynamic` receiver that nothing declares: on a `JsSelect`,
    /// `recv.selectDynamic("name")` (`updateDynamic` as an assignment's left side); on the
    /// `CallClosure` of one, `recv.applyDynamic("name")(args*)`.
    Dynamic(Name),
    /// An `ObjLit` of `js.Dynamic.literal(a = 1, ..)`: `literal.applyDynamicNamed("apply")` of the
    /// (key, value) pairs of the children; the symbol is `literal`'s.
    DynamicLiteral(SymId),
    /// An `ObjLit` that stands for an anonymous JavaScript object, `new T { val x = .. }` of JS
    /// traits, for which the typer makes no class: the anonymous class over the parents' type,
    /// its members the (key, value) pairs of the children, and `new` of it. A key given twice is a
    /// val after an assignment to an optional var, whose first pair (`Unit`) holds its place only.
    JsObject(TypeId),
    /// A `Block` that stands for a case class's `recv.copy[T..](args)`: its first statement
    /// binds the receiver, its value is the `new` (the type arguments the copy's) whose
    /// arguments are the ones given and, for the others, the receiver's fields, which the
    /// pickle writes as the `copy$default$N` getters of the receiver.
    CaseCopy,
    /// A given the typer synthesized for the type (a `ClassTag`, `<:<`, `ValueOf`, `CanEqual`,
    /// `Type`, `NotGiven`): scalac's `Synthesizer` tree for it; the node's children are teq's.
    Evidence(TypeId),
    /// A partial function literal, the `Js` of its `applyOrElse` and `isDefinedAt`: the
    /// closure over the first one's first parameter whose body is its match without the last
    /// case, the default's.
    PartialFunction,
    /// The `Quotes` the given search found inside a splice of a quote (an enclosing method's):
    /// the splice's own context parameter, which scalac's search finds there
    /// (`${ (using contextual$2: Quotes) => helper('{ y }.apply(contextual$2)) }`).
    SpliceQuotes,
    /// A `New` of a case class the source wrote as an application of the class's name
    /// (`Some(x)`): the companion's `apply`, which scalac's typer selects there.
    CaseApply,
    /// A `New` of a Java annotation, an annotation's tree: its interface's unsigned constructor
    /// applied to the elements given, each a named argument (`Applications.isJavaAnnotConstr`).
    JavaAnnotation,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wrap {
    /// `(e: T)`.
    Ascribed(TypeId),
    /// `e.asInstanceOf[T]` where the cast leaves the node as it was.
    Cast(TypeId),
    /// `(e: @unchecked)`.
    Unchecked,
    /// `e: _*`, passed to a repeated parameter of the element type.
    Splice(TypeId),
    /// A named argument, `name = e`.
    Named(Name),
    /// `e.name` of a member the typer wrote as `e` itself (`toString` of a `String`), with the
    /// span of the whole selection, which the node's own span does not cover.
    Member(Name, crate::source::Span),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PatForm {
    /// A `Seq` pattern's extractor: `unapplySeq` of the type's companion (`List`, `Seq`,
    /// `Array`, ...).
    Seq(TypeId),
    /// A `Seq` pattern that is the elements of the extractor around it: a case class's repeated
    /// field, an `unapplySeq`'s sequence.
    Elements,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Local {
    /// Modifiers the symbol does not carry (`mods::INLINE` of a local val).
    pub mods: crate::ast::Mods,
    /// The type of a temporary the typer gave `Any` in the place of its value's (a hoisted
    /// argument's or operand's).
    pub ty: Option<TypeId>,
}

/// An inline method's call as it was typed, before its expansion: scalac's pickle keeps an
/// ordinary inline call as the call, and a transparent one's expansion as `INLINED` with the
/// origin and the bindings.
#[derive(Clone, Debug)]
pub struct InlineCall {
    pub callee: SymId,
    pub recv: Option<TExprId>,
    /// The arguments in clause order, as they were typed at the call.
    pub args: Vec<TExprId>,
    /// The method's type arguments.
    pub targs: TList,
    pub transparent: bool,
    /// Whether the expansion is a `Block` whose statements bind the receiver and arguments
    /// (the `INLINED`'s bindings) and whose value is the expansion of the body.
    pub binds: bool,
    /// The call's source and span, which the pickle's call of an ordinary inline method takes.
    pub site: Option<(crate::source::FileId, crate::source::Span)>,
}

/// A call the typer replaced by what it computes, which scalac's pickle holds as the call: of a
/// member the typer gives every tuple (`head`, `tail`, `size`, `apply`, `++`, `zip`, `toList`,
/// `*:`, inline methods of scalac's `Tuple`), or of a lambda applied to paths and literals,
/// whose body the typer instantiated (`apply` of the function value): the member's name, the
/// receiver and the arguments as they were typed.
#[derive(Clone, Debug)]
pub struct BuiltinCall {
    pub name: Name,
    pub recv: TExprId,
    pub args: Vec<TExprId>,
}

impl Capture {
    pub fn new(owned: Vec<bool>, annotated: Vec<bool>) -> Capture {
        Capture { owned, annotated, ..Capture::default() }
    }

    /// A worker's capture over the same owned files, its records its own until the merge,
    /// the published ones every worker's.
    pub fn attached(&self) -> Capture {
        Capture { owned: self.owned.clone(), annotated: self.annotated.clone(), published: self.published.clone(), ..Capture::default() }
    }

    fn read_published(&self) -> std::sync::RwLockReadGuard<'_, Published> {
        self.published.read().unwrap_or_else(|e| e.into_inner())
    }

    /// The records of the node `e`: this capture's, else those another worker published.
    pub fn records_of(&self, e: TExprId) -> NodeRecords {
        let own = self.own_records(e);
        if !own.is_empty() {
            return own;
        }
        let p = self.read_published();
        NodeRecords {
            targs: p.targs.get(&e).copied(),
            form: p.forms.get(&e).copied(),
            wraps: p.wraps.get(&e).cloned().unwrap_or_default(),
            inline_calls: p.inline_calls.get(&e).cloned().unwrap_or_default(),
            builtin: p.builtin_calls.get(&e).cloned(),
            receiver: p.receivers.get(&e).copied(),
            evidence: p.evidence.get(&e).cloned().unwrap_or_default(),
            tree: p.trees.get(&e).copied(),
        }
    }

    /// The site of the outermost inline call the node `e` is the expansion of: this capture's
    /// record, else a published one.
    pub fn inline_site_of(&self, e: TExprId) -> Option<(crate::source::FileId, crate::source::Span)> {
        let outer = |calls: &Vec<InlineCall>| calls.last().and_then(|c| c.site);
        match self.inline_calls.get(&e) {
            Some(calls) => outer(calls),
            None => self.read_published().inline_calls.get(&e).and_then(outer),
        }
    }

    /// The record of the pattern `p`: this capture's, else a published one.
    pub fn pat_of(&self, p: TPatId) -> Option<PatForm> {
        self.pats.get(&p).copied().or_else(|| self.read_published().pats.get(&p).copied())
    }

    /// The record of the local `s`: this capture's, else a published one.
    pub fn local_of(&self, s: SymId) -> Option<Local> {
        self.locals.get(&s).copied().or_else(|| self.read_published().locals.get(&s).copied())
    }

    /// Publishes the records the typing of `unit` made since `mark` (a quote's body's) for
    /// every worker's copier.
    pub fn publish(&mut self, unit: FileId, mark: (usize, usize)) {
        let Some(keys) = self.made.get(&unit) else { return };
        let Some(keys) = keys.get(mark.0..).filter(|k| !k.is_empty()) else { return };
        self.publishing = true;
        let mut p = self.published.write().unwrap_or_else(|e| e.into_inner());
        for &k in keys {
            match k {
                Key::Targs(e) => p.targs.extend(self.targs.get(&e).map(|&l| (e, l))),
                Key::Form(e) => p.forms.extend(self.forms.get(&e).map(|&f| (e, f))),
                Key::Wrap(e) => p.wraps.extend(self.wraps.get(&e).map(|w| (e, w.clone()))),
                Key::Inline(e) => p.inline_calls.extend(self.inline_calls.get(&e).map(|c| (e, c.clone()))),
                Key::Builtin(e) => p.builtin_calls.extend(self.builtin_calls.get(&e).map(|c| (e, c.clone()))),
                Key::Receiver(e) => p.receivers.extend(self.receivers.get(&e).map(|&r| (e, r))),
                Key::Evidence(e) => p.evidence.extend(self.evidence.get(&e).map(|v| (e, v.clone()))),
                Key::Tree(e) => p.trees.extend(self.trees.get(&e).map(|&t| (e, t))),
                Key::Pat(x) => p.pats.extend(self.pats.get(&x).map(|&f| (x, f))),
                Key::Local(s) => p.locals.extend(self.locals.get(&s).map(|&l| (s, l))),
                Key::Class(_) => {}
            }
        }
    }



    #[inline]
    pub fn owns(&self, f: FileId) -> bool {
        self.owned.get(f.0 as usize).copied().unwrap_or(false)
    }

    pub fn annotates(&self, f: FileId) -> bool {
        self.annotated.get(f.0 as usize).copied().unwrap_or(false)
    }

    fn made_by(&mut self, unit: FileId, key: Key) {
        self.made.entry(unit).or_default().push(key);
    }

    fn changed_by(&mut self, unit: FileId, prior: Prior) {
        let made = self.made.get(&unit).map_or(0, |k| k.len()) as u32;
        self.changed.entry(unit).or_default().push((made, prior));
    }

    pub fn targs(&mut self, unit: FileId, e: TExprId, targs: TList) {
        match self.targs.insert(e, targs) {
            None => self.made_by(unit, Key::Targs(e)),
            Some(old) if old != targs => self.changed_by(unit, Prior::Targs(e, old)),
            Some(_) => {}
        }
    }

    pub fn form(&mut self, unit: FileId, e: TExprId, f: Form) {
        match self.forms.insert(e, f) {
            None => self.made_by(unit, Key::Form(e)),
            Some(old) if old != f => self.changed_by(unit, Prior::Form(e, old)),
            Some(_) => {}
        }
    }

    /// A layer around `e`. A name or a sequence argument is one per argument, which a node
    /// typed once and passed again (an overload tried with arguments typed before it) keeps.
    pub fn wrap(&mut self, unit: FileId, e: TExprId, w: Wrap) {
        let prior = self.wraps.get(&e).cloned();
        let layers = self.wraps.entry(e).or_default();
        let once = |l: &Wrap| matches!((l, w), (Wrap::Named(_), Wrap::Named(_)) | (Wrap::Splice(_), Wrap::Splice(_)));
        match layers.iter_mut().find(|l| once(l)) {
            Some(l) => *l = w,
            None => layers.push(w),
        }
        match prior {
            None => self.made_by(unit, Key::Wrap(e)),
            Some(old) if self.wraps.get(&e) != Some(&old) => self.changed_by(unit, Prior::Wraps(e, old)),
            Some(_) => {}
        }
    }

    /// The layers `ws` around `e` in the place of what it had.
    fn set_wraps(&mut self, unit: FileId, e: TExprId, ws: Vec<Wrap>) {
        match self.wraps.insert(e, ws) {
            None => self.made_by(unit, Key::Wrap(e)),
            Some(old) => self.changed_by(unit, Prior::Wraps(e, old)),
        }
    }

    pub fn pat(&mut self, unit: FileId, p: TPatId, f: PatForm) {
        match self.pats.insert(p, f) {
            None => self.made_by(unit, Key::Pat(p)),
            Some(old) if old != f => self.changed_by(unit, Prior::Pat(p, old)),
            Some(_) => {}
        }
    }

    /// The record of the local `s`, to change: what it was is kept for a drop of an attempt.
    pub fn local(&mut self, unit: FileId, s: SymId) -> &mut Local {
        match self.locals.get(&s).copied() {
            None => self.made_by(unit, Key::Local(s)),
            Some(old) => self.changed_by(unit, Prior::Local(s, old)),
        }
        self.locals.entry(s).or_default()
    }

    pub fn inline_call(&mut self, unit: FileId, e: TExprId, call: InlineCall) {
        match self.inline_calls.get(&e).cloned() {
            None => self.made_by(unit, Key::Inline(e)),
            Some(old) => self.changed_by(unit, Prior::Inline(e, old)),
        }
        self.inline_calls.entry(e).or_default().push(call);
    }

    /// The calls `calls` for the chain of `e` in the place of what it had.
    fn set_inline_calls(&mut self, unit: FileId, e: TExprId, calls: Vec<InlineCall>) {
        match self.inline_calls.insert(e, calls) {
            None => self.made_by(unit, Key::Inline(e)),
            Some(old) => self.changed_by(unit, Prior::Inline(e, old)),
        }
    }

    /// The node `to` takes the place of `from`, whose node it is given: what describes the node
    /// (its type arguments, its form, the call it computes) is `from`'s, the layers around it
    /// and the inline calls it stands for are `from`'s inside those `to` had, and `from` keeps
    /// none.
    pub fn moved(&mut self, unit: FileId, from: TExprId, to: TExprId) {
        let old = self.own_records(to);
        let new = self.own_records(from);
        if old.is_empty() && new.is_empty() {
            return;
        }
        if let Some(l) = new.targs {
            self.unset(unit, Key::Targs(from), Prior::Targs(from, l));
        }
        if let Some(f) = new.form {
            self.unset(unit, Key::Form(from), Prior::Form(from, f));
        }
        if !new.wraps.is_empty() {
            self.unset(unit, Key::Wrap(from), Prior::Wraps(from, new.wraps.clone()));
        }
        if !new.inline_calls.is_empty() {
            self.unset(unit, Key::Inline(from), Prior::Inline(from, new.inline_calls.clone()));
        }
        if let Some(b) = new.builtin.clone() {
            self.unset(unit, Key::Builtin(from), Prior::Builtin(from, b));
        }
        if let Some(r) = new.receiver {
            self.unset(unit, Key::Receiver(from), Prior::Receiver(from, r));
        }
        if !new.evidence.is_empty() {
            self.unset(unit, Key::Evidence(from), Prior::Evidence(from, new.evidence.clone()));
        }
        if let Some(t) = new.tree {
            self.unset(unit, Key::Tree(from), Prior::Tree(from, t));
        }
        let wraps: Vec<Wrap> = new.wraps.iter().chain(&old.wraps).copied().collect();
        let calls: Vec<InlineCall> = new.inline_calls.iter().chain(&old.inline_calls).cloned().collect();
        match (new.targs, old.targs) {
            (Some(l), _) => self.targs(unit, to, l),
            (None, Some(l)) => self.unset(unit, Key::Targs(to), Prior::Targs(to, l)),
            (None, None) => {}
        }
        match (new.form, old.form) {
            (Some(f), _) => self.form(unit, to, f),
            (None, Some(f)) => self.unset(unit, Key::Form(to), Prior::Form(to, f)),
            (None, None) => {}
        }
        match (new.builtin, old.builtin) {
            (Some(b), _) => self.builtin_call(unit, to, b),
            (None, Some(b)) => self.unset(unit, Key::Builtin(to), Prior::Builtin(to, b)),
            (None, None) => {}
        }
        match (new.receiver, old.receiver) {
            (Some(r), _) => self.receiver(unit, to, r),
            (None, Some(r)) => self.unset(unit, Key::Receiver(to), Prior::Receiver(to, r)),
            (None, None) => {}
        }
        match (new.evidence.is_empty(), old.evidence.is_empty()) {
            (false, _) => self.evidence(unit, to, new.evidence),
            (true, false) => self.unset(unit, Key::Evidence(to), Prior::Evidence(to, old.evidence)),
            (true, true) => {}
        }
        match (new.tree, old.tree) {
            (Some(t), _) => self.tree(unit, to, t),
            (None, Some(t)) => self.unset(unit, Key::Tree(to), Prior::Tree(to, t)),
            (None, None) => {}
        }
        if !wraps.is_empty() {
            self.set_wraps(unit, to, wraps);
        }
        if !calls.is_empty() {
            self.set_inline_calls(unit, to, calls);
        }
    }

    /// Drops the record under `k`, which `prior` was: put back by a drop of the attempt.
    fn unset(&mut self, unit: FileId, k: Key, prior: Prior) {
        self.drop_key(k);
        self.changed_by(unit, prior);
    }

    fn own_records(&self, e: TExprId) -> NodeRecords {
        NodeRecords {
            targs: self.targs.get(&e).copied(),
            form: self.forms.get(&e).copied(),
            wraps: self.wraps.get(&e).cloned().unwrap_or_default(),
            inline_calls: self.inline_calls.get(&e).cloned().unwrap_or_default(),
            builtin: self.builtin_calls.get(&e).cloned(),
            receiver: self.receivers.get(&e).copied(),
            evidence: self.evidence.get(&e).cloned().unwrap_or_default(),
            tree: self.trees.get(&e).copied(),
        }
    }

    /// `to`, a copy of `from`, takes the records of `from`.
    pub fn copy(&mut self, unit: FileId, from: TExprId, to: TExprId) {
        if let Some(&t) = self.targs.get(&from) {
            self.targs(unit, to, t);
        }
        if let Some(&f) = self.forms.get(&from) {
            self.form(unit, to, f);
        }
        if let Some(w) = self.wraps.get(&from).cloned() {
            self.set_wraps(unit, to, w);
        }
        if let Some(c) = self.inline_calls.get(&from).cloned() {
            self.set_inline_calls(unit, to, c);
        }
        if let Some(c) = self.builtin_calls.get(&from).cloned() {
            self.builtin_call(unit, to, c);
        }
        if let Some(&r) = self.receivers.get(&from) {
            self.receiver(unit, to, r);
        }
        if let Some(v) = self.evidence.get(&from).cloned() {
            self.evidence(unit, to, v);
        }
        if let Some(&t) = self.trees.get(&from) {
            self.tree(unit, to, t);
        }
    }

    /// The published records under `keys` as this capture has them now: dropped where it
    /// dropped them, as it put them back or zonked them where it has them.
    pub fn sync_published(&self, keys: &[Key]) {
        if !self.publishing || keys.is_empty() {
            return;
        }
        let mut p = self.published.write().unwrap_or_else(|e| e.into_inner());
        for &k in keys {
            if !p.has(k) {
                continue;
            }
            match k {
                Key::Targs(e) => sync(&mut p.targs, e, self.targs.get(&e).copied()),
                Key::Form(e) => sync(&mut p.forms, e, self.forms.get(&e).copied()),
                Key::Wrap(e) => sync(&mut p.wraps, e, self.wraps.get(&e).cloned()),
                Key::Inline(e) => sync(&mut p.inline_calls, e, self.inline_calls.get(&e).cloned()),
                Key::Builtin(e) => sync(&mut p.builtin_calls, e, self.builtin_calls.get(&e).cloned()),
                Key::Receiver(e) => sync(&mut p.receivers, e, self.receivers.get(&e).copied()),
                Key::Evidence(e) => sync(&mut p.evidence, e, self.evidence.get(&e).cloned()),
                Key::Tree(e) => sync(&mut p.trees, e, self.trees.get(&e).copied()),
                Key::Pat(x) => sync(&mut p.pats, x, self.pats.get(&x).copied()),
                Key::Local(s) => sync(&mut p.locals, s, self.locals.get(&s).copied()),
                Key::Class(_) => {}
            }
        }
    }

    /// The published records no record of this capture stands behind: of a typing dropped, a
    /// file retyped, a node compacted, which should have gone with it. What the census counts as
    /// orphans once the workers' captures are one.
    pub fn published_orphans(&self) -> usize {
        let p = self.read_published();
        p.targs.keys().filter(|e| !self.targs.contains_key(e)).count()
            + p.forms.keys().filter(|e| !self.forms.contains_key(e)).count()
            + p.wraps.keys().filter(|e| !self.wraps.contains_key(e)).count()
            + p.inline_calls.keys().filter(|e| !self.inline_calls.contains_key(e)).count()
            + p.builtin_calls.keys().filter(|e| !self.builtin_calls.contains_key(e)).count()
            + p.receivers.keys().filter(|e| !self.receivers.contains_key(e)).count()
            + p.evidence.keys().filter(|e| !self.evidence.contains_key(e)).count()
            + p.trees.keys().filter(|e| !self.trees.contains_key(e)).count()
            + p.pats.keys().filter(|x| !self.pats.contains_key(x)).count()
            + p.locals.keys().filter(|s| !self.locals.contains_key(s)).count()
    }

    /// Drops the records the typing of `file` made, the published ones among them: its bodies
    /// are typed again.
    pub fn forget(&mut self, file: FileId) {
        self.changed.remove(&file);
        self.block_stmts.retain(|&(f, _), _| f != file);
        self.annotations.retain(|&(f, _), _| f != file);
        let Some(keys) = self.made.remove(&file) else { return };
        for &k in &keys {
            self.drop_key(k);
        }
        self.sync_published(&keys);
    }

    /// Where the typing of `unit` stands: how many records it made and how many it changed,
    /// for `drop_since`.
    pub fn mark(&self, unit: FileId) -> (usize, usize) {
        (self.made.get(&unit).map_or(0, |k| k.len()), self.changed.get(&unit).map_or(0, |k| k.len()))
    }

    /// Undoes what the typing of `unit` did after `mark`, newest first: a record it made is
    /// dropped, one it changed or deleted takes back its value. How many it made.
    pub fn drop_since(&mut self, unit: FileId, mark: (usize, usize)) -> usize {
        let (made, changed) = mark;
        let priors: Vec<(u32, Prior)> = match self.changed.get_mut(&unit) {
            Some(p) if p.len() > changed => p.drain(changed..).collect(),
            _ => Vec::new(),
        };
        let keys: Vec<Key> = match self.made.get_mut(&unit) {
            Some(k) if k.len() > made => k.drain(made..).collect(),
            _ => Vec::new(),
        };
        let mut touched: Vec<Key> = keys.clone();
        // A change came after the records made before its stamp and before the others.
        let mut undone = keys.len();
        for (stamp, prior) in priors.into_iter().rev() {
            let after = (stamp as usize).saturating_sub(made);
            while undone > after {
                undone -= 1;
                self.drop_key(keys[undone]);
            }
            touched.push(prior.key());
            self.put_back(prior);
        }
        while undone > 0 {
            undone -= 1;
            self.drop_key(keys[undone]);
        }
        // The published table as the records now stand, in the same call: the keys that tell
        // which records the table holds are gone once it returns.
        self.sync_published(&touched);
        self.discarded += keys.len() as u64;
        keys.len()
    }

    fn put_back(&mut self, prior: Prior) {
        match prior {
            Prior::Targs(e, l) => {
                self.targs.insert(e, l);
            }
            Prior::Form(e, f) => {
                self.forms.insert(e, f);
            }
            Prior::Wraps(e, ws) => {
                self.wraps.insert(e, ws);
            }
            Prior::Inline(e, calls) => {
                self.inline_calls.insert(e, calls);
            }
            Prior::Builtin(e, call) => {
                self.builtin_calls.insert(e, call);
            }
            Prior::Receiver(e, r) => {
                self.receivers.insert(e, r);
            }
            Prior::Evidence(e, v) => {
                self.evidence.insert(e, v);
            }
            Prior::Tree(e, t) => {
                self.trees.insert(e, t);
            }
            Prior::Pat(p, f) => {
                self.pats.insert(p, f);
            }
            Prior::Local(s, l) => {
                self.locals.insert(s, l);
            }
        }
    }

    /// Whether there is a record under `k`.
    pub fn has(&self, k: Key) -> bool {
        match k {
            Key::Targs(e) => self.targs.contains_key(&e),
            Key::Form(e) => self.forms.contains_key(&e),
            Key::Wrap(e) => self.wraps.contains_key(&e),
            Key::Inline(e) => self.inline_calls.contains_key(&e),
            Key::Builtin(e) => self.builtin_calls.contains_key(&e),
            Key::Receiver(e) => self.receivers.contains_key(&e),
            Key::Evidence(e) => self.evidence.contains_key(&e),
            Key::Tree(e) => self.trees.contains_key(&e),
            Key::Pat(p) => self.pats.contains_key(&p),
            Key::Local(s) => self.locals.contains_key(&s),
            Key::Class(c) => self.expansion_classes.contains_key(&c),
        }
    }

    /// Drops the records under `k`.
    pub fn drop_key(&mut self, k: Key) {
        match k {
            Key::Targs(e) => {
                self.targs.remove(&e);
            }
            Key::Form(e) => {
                self.forms.remove(&e);
            }
            Key::Wrap(e) => {
                self.wraps.remove(&e);
            }
            Key::Inline(e) => {
                self.inline_calls.remove(&e);
            }
            Key::Builtin(e) => {
                self.builtin_calls.remove(&e);
            }
            Key::Receiver(e) => {
                self.receivers.remove(&e);
            }
            Key::Evidence(e) => {
                self.evidence.remove(&e);
            }
            Key::Tree(e) => {
                self.trees.remove(&e);
            }
            Key::Pat(p) => {
                self.pats.remove(&p);
            }
            Key::Local(s) => {
                self.locals.remove(&s);
            }
            Key::Class(c) => {
                self.expansion_classes.remove(&c);
            }
        }
    }

    pub fn builtin_call(&mut self, unit: FileId, e: TExprId, call: BuiltinCall) {
        match self.builtin_calls.insert(e, call) {
            None => self.made_by(unit, Key::Builtin(e)),
            Some(old) => self.changed_by(unit, Prior::Builtin(e, old)),
        }
    }

    pub fn receiver(&mut self, unit: FileId, e: TExprId, r: TExprId) {
        match self.receivers.insert(e, r) {
            None => self.made_by(unit, Key::Receiver(e)),
            Some(old) if old != r => self.changed_by(unit, Prior::Receiver(e, old)),
            Some(_) => {}
        }
    }

    pub fn evidence(&mut self, unit: FileId, e: TExprId, v: Vec<TExprId>) {
        match self.evidence.insert(e, v) {
            None => self.made_by(unit, Key::Evidence(e)),
            Some(old) => self.changed_by(unit, Prior::Evidence(e, old)),
        }
    }

    pub fn tree(&mut self, unit: FileId, e: TExprId, t: (u32, u32)) {
        match self.trees.insert(e, t) {
            None => self.made_by(unit, Key::Tree(e)),
            Some(old) if old != t => self.changed_by(unit, Prior::Tree(e, old)),
            Some(_) => {}
        }
    }

    pub fn expansion_class(&mut self, unit: FileId, c: ClassId) {
        if self.expansion_classes.insert(c, ()).is_none() {
            self.made_by(unit, Key::Class(c));
        }
    }

    /// The copies of other workers' nodes take their records, once every worker's are here.
    pub fn settle_copies(&mut self) {
        for (from, to, unit) in std::mem::take(&mut self.pending) {
            self.copy(unit, from, to);
        }
    }

    /// Takes over another worker's records, whose ids the merge renumbers after.
    pub fn absorb(&mut self, o: &mut Capture) {
        self.pending.append(&mut o.pending);
        self.targs.extend(o.targs.drain());
        self.forms.extend(o.forms.drain());
        self.wraps.extend(o.wraps.drain());
        self.pats.extend(o.pats.drain());
        self.locals.extend(o.locals.drain());
        self.inline_calls.extend(o.inline_calls.drain());
        self.builtin_calls.extend(o.builtin_calls.drain());
        self.receivers.extend(o.receivers.drain());
        self.evidence.extend(o.evidence.drain());
        self.trees.extend(o.trees.drain());
        self.expansion_classes.extend(o.expansion_classes.drain());
        self.block_stmts.extend(o.block_stmts.drain());
        self.annotations.extend(o.annotations.drain());
        for (f, keys) in o.made.drain() {
            self.made.entry(f).or_default().extend(keys);
        }
        self.publishing |= o.publishing;
        self.copies_checked += o.copies_checked;
        self.copies_lost += o.copies_lost;
        self.lost_at.append(&mut o.lost_at);
        self.lost_at.sort();
        self.lost_at.truncate(LOST_PLACES);
    }

    /// A record the copier did not carry to a copy, at `place`.
    pub fn lost(&mut self, place: String) {
        self.copies_lost += 1;
        if self.lost_at.len() < LOST_PLACES {
            self.lost_at.push(place);
        }
    }

    /// How many records there are.
    pub fn records(&self) -> usize {
        self.targs.len() + self.forms.len() + self.wraps.len() + self.pats.len() + self.locals.len() + self.inline_calls.len() + self.builtin_calls.len() + self.receivers.len() + self.evidence.len() + self.trees.len()
    }

    pub fn held(&self) -> usize {
        use crate::held::{array, table};
        table(&self.targs)
            + table(&self.forms)
            + table(&self.wraps)
            + self.wraps.values().map(array).sum::<usize>()
            + table(&self.pats)
            + table(&self.locals)
            + table(&self.inline_calls)
            + table(&self.expansion_classes)
            + table(&self.builtin_calls)
            + self.builtin_calls.values().map(|c| array(&c.args)).sum::<usize>()
            + table(&self.receivers)
            + table(&self.evidence)
            + self.evidence.values().map(array).sum::<usize>()
            + table(&self.trees)
            + self.inline_calls.values().map(|c| array(c) + c.iter().map(|c| array(&c.args)).sum::<usize>()).sum::<usize>()
            + table(&self.made)
            + self.made.values().map(array).sum::<usize>()
            + table(&self.block_stmts)
            + self.block_stmts.values().map(array).sum::<usize>()
            + table(&self.annotations)
            + table(&self.changed)
            + self.changed.values().map(array).sum::<usize>()
            + array(&self.pending)
            + array(&self.owned)
            + self.read_published().held()
    }
}
