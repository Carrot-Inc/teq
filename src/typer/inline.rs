//! `inline` methods, expanded at their call sites. A call of an inline method types the
//! method's body again in the scope of its definition, with the parameters bound to the
//! arguments: a by-value parameter to a local holding its argument, an `inline` or by-name
//! parameter to the argument expression itself, the type parameters to the type arguments and
//! `this` to the receiver. Inside such an expansion `inline if` and `inline match` reduce on
//! constants and static types, and the members of `scala.compiletime` are evaluated by name.
//! The body is also typed once at its definition and stored, as scalac types it
//! (`inline_definition.rs`), which the expansion does not read yet.

use super::apply::{ArgList, ArgSrc, MethodCall};
use super::profile::{About, Kind, Outcome, Part};
use super::{Env, ExtScope, Frame, ResolvedImport, Worker};
use crate::ast::{mods, DefKind, Expr, ExprId, ListRef, Pat, PatId, TyExpr, TyExprId};
use crate::intern::{FxMap, Name};
use crate::names;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::sync::Arc;

/// scalac's default `-Xmax-inlines`; `--max-inlines` raises it.
pub const MAX_INLINES: u32 = 32;
/// The interpreter's budget for folding one argument.
const FOLD_STEPS: u64 = 50_000;

impl InlineState {
    /// A worker's state with the build's settings: the nesting limit and the link-time
    /// properties.
    pub fn worker(main: &InlineState) -> InlineState {
        InlineState {
            max_depth: main.max_depth,
            production: main.production,
            outlines: main.outlines,
            es_modules: main.es_modules,
            ..InlineState::new()
        }
    }

    pub fn new() -> InlineState {
        InlineState { max_depth: MAX_INLINES, ..Default::default() }
    }
}

/// The Scala.js release whose linker the link-time properties answer as.
const SCALAJS_VERSION: &str = "1.21.0";

#[derive(Default)]
pub struct InlineState {
    pub depth: u32,
    /// Whether the last expansion's value is a block of the receiver's and the arguments'
    /// bindings around the body's expansion, for the capture (`capture::InlineCall::binds`).
    pub last_binds: bool,
    /// How many expansions by substitution are under way, for the count of the calls a walk
    /// hands to the retype path (`substitution::counts`).
    pub walks: u32,
    /// The nesting limit, `MAX_INLINES` unless `--max-inlines` says otherwise.
    pub max_depth: u32,
    /// The build's answers to Scala.js's link-time properties: `--release` and `--split`.
    pub production: bool,
    /// The build writes JavaScript, whose emitter outlines expansions of one shape: the typer
    /// records the expansions and their leaves (`Program::expansions`).
    pub outlines: bool,
    pub es_modules: bool,
    /// The expansions under way, outermost first.
    pub sites: Vec<InlineSite>,
    /// The environment of the outermost call site, where `summonInline` searches.
    pub site_env: Option<Arc<Env>>,
    /// The imports of the blocks of the body under expansion that enclose the point being
    /// typed, each with the frame of its block: the expanded body stands at the call site with
    /// them, so its searches see them, each import beside the givens of its block.
    pub body_imports: Vec<(usize, ResolvedImport)>,
    /// The locals of value imports in the blocks being typed (`import v.{m as a}`), each
    /// standing for its selection in `args` until its block ends.
    pub import_aliases: Vec<SymId>,
    /// The blocks being typed that define a local inline method, innermost last, each with the
    /// statement being typed: what a call before a definition finds its definition's imports by
    /// (`Worker::demand_local_definition`).
    pub blocks: Vec<BlockCursor>,
    /// The local inline methods called before their definitions across a val that an import
    /// between the two reads from (`val first = g(); val source = ..; import source.k`), each
    /// with scalac's E039 for the call.
    pub forward_refs: FxMap<SymId, String>,
    /// The index each record's copies read (`Worker::stored_index_of`), with the record it was
    /// made from.
    pub(super) stored_indexes: FxMap<SymId, (Arc<crate::tir::InlineDefinition>, Arc<super::inline_definition::StoredIndex>)>,
    /// The empty substitution the copies of bodies without type parameters share
    /// (`Worker::begin_demand`).
    pub(super) no_subst: Arc<Subst>,
    /// The checked record a walk's kept call was found to expand from, for the expansion of that
    /// call that follows: a checked record has no failed state, no abstract member and no other
    /// entry decision (`Worker::resolved`).
    pub(super) entry: Option<(SymId, Arc<InlineDefinition>)>,
    /// The walks' states the expansions by substitution ended with, kept for the next ones
    /// (`Worker::take_walk`).
    pub(super) spare_walks: Vec<Box<super::substitution::Walk>>,
    /// The lists a reduced match's patterns are recorded as stored with
    /// (`Worker::store_case_pats`), kept for the next.
    pub(super) stored_lists: (Vec<TPatId>, Vec<TestId>),
    /// The locals in scope where each nested expansion under way was called, outermost
    /// first: a class in an inner expansion may capture them.
    pub outer_locals: Vec<Vec<SymId>>,
    /// Where the frames of each body under expansion begin in the environment it is typed in,
    /// innermost last: the givens of the frames above are the body's own.
    pub body_frames: Vec<usize>,
    /// The scopes of the bodies of the enclosing expansions around the calls of the nested
    /// expansions under way, as the givens each scope defines and the imports it holds,
    /// outermost first: the expanded code stands in those scopes, where `summonInline`
    /// searches (`summon_at_site`).
    pub outer_scopes: Vec<BodyScope>,
    /// Set by a given search around the expansion of an inline candidate (`InlineSite::at_end`).
    pub site_at_end: bool,
    /// The type parameters of the methods being expanded, each bound to its type argument, and
    /// the type variables that the patterns of an `inline match` bound. Changed through
    /// `bind_tparam` and `truncate_tparams`, which keep `tparams_gen` moving.
    pub tparams: Vec<(TParamId, TypeId)>,
    pub tparams_gen: u64,
    /// `tparams` as one substitution, innermost binding of a parameter first, for the
    /// generation it was built at.
    pub tparams_subst: Option<(u64, Arc<Subst>)>,
    /// Parameters that stand for their argument expression: `inline` and by-name parameters,
    /// and by-value ones whose argument is stable. A use copies the expression.
    pub args: FxMap<SymId, InlineArg>,
    /// The parameters of the methods being expanded and the locals standing for them, which a
    /// library body names by symbol.
    pub param_proxies: FxMap<SymId, SymId>,
    /// The same as paths: a type naming a parameter (`x.type`) names its local instead.
    pub param_paths: Vec<(SymId, TypeId)>,
    /// The argument each parameter proxy of the expansions under way stands for, which a
    /// macro reads as the code behind `'x` (`underlyingArgument`), with the parameter's
    /// declared type (`proxy_declared`).
    pub param_bindings: FxMap<SymId, (TExprId, TypeId)>,
    /// The copies of substituted arguments made in the expansions under way, each with the
    /// proxy it stands for, by which an argument or a scrutinee that names a parameter is
    /// known (`proxy_declared`).
    pub copies: Vec<(TExprId, SymId)>,
    /// The receiver of each expansion: the class whose `this` it is, the local holding it
    /// unless the receiver is `this` itself, and its type.
    pub this: Vec<(ClassId, Option<SymId>, TypeId)>,
    /// The receivers that are paths, whose `C.this` in a type of the body is the path, as the
    /// caller saw the member's signature.
    pub this_paths: Vec<(ClassId, TypeId)>,
    /// The type variables of the pattern being matched by an `inline match`.
    pub pat_vars: Vec<(Name, TypeId)>,
    /// What the last `erasedValue[T]` produced: a scrutinee without a value.
    pub erased: Option<TExprId>,
    /// The binders of the inline matches being reduced that an erased scrutinee binds
    /// (`InlineReducer.reduceInlineMatch`'s `unusable`): no value, no read.
    pub unusable: Vec<SymId>,
    /// How many retained inline bodies are being typed at their definition.
    pub retained: u32,
    /// How many inline bodies are being typed by the definition check (`inline_definition.rs`),
    /// which keeps the calls of inline methods and intrinsics for the expansion and types every
    /// branch of an `inline if` or `inline match`.
    pub checking: u32,
    /// What the bodies under the definition check keep for the expansion, innermost last.
    pub notes: Vec<super::inline_definition::Notes>,
    /// Set while the interpreter folds an expression, which must not start another fold.
    pub folding: bool,
    /// Set while a constant is read off a tree as it stands: no val's body is typed for it.
    pub as_typed: bool,
    /// The literals the interpreter evaluated arguments to, with their copies: constants of
    /// the output, not of the language, so an `inline if` or `inline val` does not see them.
    pub evaluated: FxMap<TExprId, ()>,
    /// Set while the `inline match` of a retained body is typed as a plain match.
    pub lenient_match: u32,
    /// The static type of the receiver of each inline member call under way, by receiver.
    pub recv_types: FxMap<TExprId, TypeId>,
    /// The static types of the arguments typed for an inline callee, which its proxies take.
    pub record_args: bool,
    pub arg_types: Vec<(TExprId, TypeId)>,
    /// How many expansions there were so far: a given search whose count moved expanded an
    /// inline given, whose result depends on the site.
    pub expansions: u64,
    /// How many types of the bodies under expansion took a type argument of the call, which
    /// tells a type test of the call's type from one of the body's.
    pub tparam_reads: std::cell::Cell<u32>,
    /// How many leaves the folding of constants met, which tells a condition on the call site's
    /// constants from one on the body's.
    pub leaf_reads: u32,
    compiletime: Option<Option<PkgId>>,
    ops_name: Option<Name>,
}

/// A scope of a body under expansion, as a search at the call site sees it: the givens it
/// defines and the imports it holds.
#[derive(Clone, Default)]
pub struct BodyScope {
    pub givens: Vec<SymId>,
    pub imports: Vec<ResolvedImport>,
}

/// What `InlineState` holds of the expansions under way, of the inline call being prepared and
/// of the retained body being typed, which a body typed on its own starts without
/// (`Worker::outside_inline`).
pub struct Expanding {
    depth: u32,
    walks: u32,
    sites: Vec<InlineSite>,
    site_env: Option<Arc<Env>>,
    body_imports: Vec<(usize, ResolvedImport)>,
    import_aliases: Vec<SymId>,
    blocks: Vec<BlockCursor>,
    outer_locals: Vec<Vec<SymId>>,
    body_frames: Vec<usize>,
    outer_scopes: Vec<BodyScope>,
    site_at_end: bool,
    tparams: Vec<(TParamId, TypeId)>,
    tparams_subst: Option<(u64, Arc<Subst>)>,
    param_proxies: FxMap<SymId, SymId>,
    param_paths: Vec<(SymId, TypeId)>,
    param_bindings: FxMap<SymId, (TExprId, TypeId)>,
    copies: Vec<(TExprId, SymId)>,
    this: Vec<(ClassId, Option<SymId>, TypeId)>,
    this_paths: Vec<(ClassId, TypeId)>,
    pat_vars: Vec<(Name, TypeId)>,
    erased: Option<TExprId>,
    retained: u32,
    checking: u32,
    notes: Vec<super::inline_definition::Notes>,
    lenient_match: u32,
    record_args: bool,
    arg_types: Vec<(TExprId, TypeId)>,
}

impl InlineState {
    /// Nothing when there is nothing to set aside, which is the case for a body the walk
    /// types. Every field is named here, so that a new one is either set aside or kept for a
    /// reason.
    pub fn set_aside(&mut self) -> Option<Expanding> {
        use std::mem::take;
        let InlineState {
            depth,
            walks,
            sites,
            site_env,
            body_imports,
            import_aliases,
            blocks,
            outer_locals,
            body_frames,
            outer_scopes,
            site_at_end,
            tparams,
            tparams_subst,
            param_proxies,
            param_paths,
            param_bindings,
            copies,
            this,
            this_paths,
            pat_vars,
            erased,
            retained,
            checking,
            notes,
            lenient_match,
            record_args,
            arg_types,
            // The build's settings.
            max_depth: _,
            production: _,
            outlines: _,
            es_modules: _,
            // Counts that only move: the substitution built for an earlier count is set aside
            // with the bindings it was built from. `outside_inline` puts back the two that
            // are read as differences.
            tparams_gen: _,
            expansions: _,
            tparam_reads: _,
            leaf_reads: _,
            // The interpreter's, whose fold is still running when a body it asks for is typed.
            folding: _,
            // Set while a constant is read off a typed tree, which asks for no body.
            as_typed: _,
            // Read as the expansion that set it returns.
            last_binds: _,
            // Kept by a local of the code that wrote the entry (a proxy, a binder, a local
            // `inline val`) or by an expression, neither of which a body typed on its own
            // names; what an `inline val` was evaluated to is read again after its body.
            args: _,
            recv_types: _,
            evaluated: _,
            // Kept for the whole compilation.
            compiletime: _,
            ops_name: _,
            forward_refs: _,
            stored_indexes: _,
            no_subst: _,
            // Taken by the expansion the walk starts next, before anything else runs.
            entry: _,
            // A stack of the matches being reduced, each truncating it to its own mark: a body
            // typed on its own leaves it as it found it.
            unusable: _,
            // Room for the next walk, whose contents no walk reads from another.
            spare_walks: _,
            stored_lists: _,
        } = self;
        let nothing = *depth == 0
            && *walks == 0
            && *retained == 0
            && *checking == 0
            && notes.is_empty()
            && *lenient_match == 0
            && !*record_args
            && !*site_at_end
            && erased.is_none()
            && site_env.is_none()
            && sites.is_empty()
            && tparams.is_empty()
            && tparams_subst.is_none()
            && this.is_empty()
            && this_paths.is_empty()
            && param_proxies.is_empty()
            && param_paths.is_empty()
            && param_bindings.is_empty()
            && copies.is_empty()
            && pat_vars.is_empty()
            && arg_types.is_empty()
            && body_imports.is_empty()
            && import_aliases.is_empty()
            && blocks.is_empty()
            && outer_locals.is_empty()
            && body_frames.is_empty()
            && outer_scopes.is_empty();
        if nothing {
            return None;
        }
        Some(Expanding {
            depth: take(depth),
            walks: take(walks),
            sites: take(sites),
            site_env: take(site_env),
            body_imports: take(body_imports),
            import_aliases: take(import_aliases),
            blocks: take(blocks),
            outer_locals: take(outer_locals),
            body_frames: take(body_frames),
            outer_scopes: take(outer_scopes),
            site_at_end: take(site_at_end),
            tparams: take(tparams),
            tparams_subst: take(tparams_subst),
            param_proxies: take(param_proxies),
            param_paths: take(param_paths),
            param_bindings: take(param_bindings),
            copies: take(copies),
            this: take(this),
            this_paths: take(this_paths),
            pat_vars: take(pat_vars),
            erased: take(erased),
            retained: take(retained),
            checking: take(checking),
            notes: take(notes),
            lenient_match: take(lenient_match),
            record_args: take(record_args),
            arg_types: take(arg_types),
        })
    }

    pub fn restore(&mut self, outer: Expanding) {
        self.depth = outer.depth;
        self.walks = outer.walks;
        self.sites = outer.sites;
        self.site_env = outer.site_env;
        self.body_imports = outer.body_imports;
        self.import_aliases = outer.import_aliases;
        self.blocks = outer.blocks;
        self.outer_locals = outer.outer_locals;
        self.body_frames = outer.body_frames;
        self.outer_scopes = outer.outer_scopes;
        self.site_at_end = outer.site_at_end;
        self.tparams = outer.tparams;
        self.tparams_subst = outer.tparams_subst;
        self.param_proxies = outer.param_proxies;
        self.param_paths = outer.param_paths;
        self.param_bindings = outer.param_bindings;
        self.copies = outer.copies;
        self.this = outer.this;
        self.this_paths = outer.this_paths;
        self.pat_vars = outer.pat_vars;
        self.erased = outer.erased;
        self.retained = outer.retained;
        self.checking = outer.checking;
        self.notes = outer.notes;
        self.lenient_match = outer.lenient_match;
        self.record_args = outer.record_args;
        self.arg_types = outer.arg_types;
    }
}

impl InlineState {
    /// What an expansion leaves behind that the next one sets before it reads, dropped at the
    /// workers' merge: a local inline val's copies, an `erasedValue` no match took, the
    /// receivers' and arguments' types of selections and applications that never expanded.
    pub fn clear_leftovers(&mut self) {
        self.copies.clear();
        self.erased = None;
        self.recv_types.clear();
        self.arg_types.clear();
    }
}

#[derive(Clone, Copy)]
pub struct InlineArg {
    pub expr: TExprId,
    pub ty: TypeId,
    /// Where the argument was written, for `codeOf` and `requireConst`.
    pub source: Option<(FileId, Span)>,
}

/// A block being typed that defines a local inline method: its frame in the environment, its
/// statements, the one being typed and where its imports begin in `Env::imports`.
#[derive(Clone, Copy)]
pub struct BlockCursor {
    pub frame: usize,
    pub file: FileId,
    pub stmts: crate::ast::ListRef,
    pub at: usize,
    pub imports_scope: usize,
}

#[derive(Clone, Copy)]
pub struct InlineSite {
    pub callee: SymId,
    pub file: FileId,
    pub span: Span,
    pub body_file: FileId,
    pub body_span: Span,
    /// The expansion supplies an inferred argument, which scalac positions at the end of the
    /// call it completes: what `Position.ofMacroExpansion` reports then.
    pub at_end: bool,
    /// The expression of a library body being typed at the call, where the call is in one: the
    /// place a diagnostic of an outer level's expansion is inlined from.
    pub call_node: Option<crate::ast::ExprId>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Intrinsic {
    ErasedValue,
    ConstValue,
    ConstValueOpt,
    ConstValueTuple,
    SummonInline,
    SummonFrom,
    SummonAll,
    Error,
    RequireConst,
    CodeOf,
    Uninitialized,
    /// `deferred`, the marker of a deferred given, typed where it is no given's right-hand
    /// side in a trait (`Worker::is_deferred_marker`).
    Deferred,
    TypeChecks,
    LinkTimeProperty,
    ConstructorOf,
}

/// What `Worker::bind_inline_call` bound for one expansion, which `unbind_inline_call` undoes.
pub(super) struct BoundCall {
    /// The vals of the proxies, each with its place: the receiver's first, then the parameters'
    /// in clause order.
    pub stmts: Vec<(u32, TStmt)>,
    this_mark: usize,
    pub this_paths_mark: usize,
    copies_mark: usize,
    /// The proxy of a stable receiver, which stands for its tree.
    this_arg: Option<SymId>,
    /// The proxy standing for `this`: a stable receiver's or the val of any other.
    pub this_proxy: Option<SymId>,
    proxies: Vec<SymId>,
    givens: Vec<SymId>,
    result_names: Option<SymId>,
    outer_proxies: Vec<(SymId, Option<SymId>)>,
    paths_mark: usize,
    /// The call's result type with its parameters' paths moved to the proxies'.
    pub ret_ty: TypeId,
    pub params: Vec<BoundParam>,
}

/// A parameter of a call bound by `Worker::bind_inline_call`.
#[derive(Clone, Copy)]
pub(super) struct BoundParam {
    pub sym: SymId,
    pub name: Name,
    pub proxy: SymId,
    /// The argument as bound: the constant it folded to, its value, or its tree.
    pub arg: TExprId,
    /// The constant of the language the argument folded to.
    pub constant: Option<LitVal>,
    pub declared: TypeId,
    local_ty: TypeId,
    pub by_name: bool,
    inline: bool,
    repeated: bool,
    is_using: bool,
    is_macro: bool,
    /// A default the expansion by substitution binds from its copy of the record's.
    pub later: bool,
}

/// What an inline method body is made of, wherever it comes from.
pub(super) struct InlineBody {
    pub file: FileId,
    pub body: ExprId,
    span: Span,
    pub transparent: bool,
    pub declared: bool,
    /// Per parameter, in clause order: whether it is an `inline` parameter.
    inline_params: Vec<bool>,
    defaults: Vec<Option<ExprId>>,
    ext_group: u32,
}

/// The scrutinee of an `inline match` as the patterns see it: the expression, its static
/// type, its constant value when it has one, and the local it is bound to once a pattern
/// needs the value.
struct Scrutinee {
    expr: TExprId,
    ty: TypeId,
    constant: Option<LitVal>,
    erased: bool,
    bound: Option<TExprId>,
    /// The erased values the whole scrutinee holds (`erased_tested`), which a field's projection
    /// is free of or not.
    erased_parts: std::rc::Rc<[TExprId]>,
}

impl<'a> Worker<'a> {
    pub fn is_inline_callee(&self, sym: SymId) -> bool {
        let info = self.syms.sym(sym);
        info.mods & mods::INLINE != 0 && matches!(info.kind, SymKind::Def | SymKind::Given)
    }

    /// An inline method that overrides or implements a member that is not inline keeps a
    /// method of its own for dynamic dispatch, as scalac retains one; its body is typed at the
    /// definition like any other, `inline if` and `inline match` as plain conditionals.
    pub fn is_retained_inline(&mut self, sym: SymId) -> bool {
        let info = self.syms.sym(sym);
        if info.mods & mods::INLINE == 0 || info.kind != SymKind::Def || info.def.is_none() {
            return false;
        }
        let Owner::Class(c) = info.owner else { return false };
        let (name, is_ext) = (info.name, info.is_extension);
        if self.is_abstract_member(sym) {
            return false;
        }
        for i in 1..self.syms.class(c).base_types.len() {
            let b = self.syms.class(c).base_types[i].0;
            let mut k = 0;
            while let Some(p) = self.inherited_member(b, name, is_ext, k) {
                k += 1;
                if self.syms.sym(p).mods & mods::INLINE == 0 {
                    return true;
                }
            }
        }
        false
    }

    pub(super) fn compiletime_pkg(&mut self) -> Option<PkgId> {
        if let Some(cached) = self.inline.compiletime {
            return cached;
        }
        let name = self.interner.intern("compiletime");
        let found = self.syms.pkg(self.b.scala_pkg).entries.get(&name).and_then(|e| e.pkg);
        self.inline.compiletime = Some(found);
        found
    }

    /// The member of `scala.compiletime` that `sym` is, evaluated by the compiler.
    pub fn intrinsic_of(&mut self, sym: SymId) -> Option<Intrinsic> {
        let Some(compiletime) = self.compiletime_pkg() else { return None };
        let info = self.syms.sym(sym);
        let pkg = match info.owner {
            Owner::Package(p) => p,
            Owner::Class(c) => match self.syms.class(c).owner {
                Owner::Package(p) => p,
                _ => return None,
            },
            Owner::Local => return None,
        };
        if let Owner::Class(c) = info.owner {
            if self.is_link_time_property(c, pkg, info.name) {
                return Some(Intrinsic::LinkTimeProperty);
            }
        }
        if self.name_ref(info.name) == "constructorOf" && self.pkg_is(pkg, "scala.scalajs.js") {
            return Some(Intrinsic::ConstructorOf);
        }
        let in_testing = pkg != compiletime && self.syms.pkg(pkg).parent == Some(compiletime) && self.name_ref(self.syms.pkg(pkg).name) == "testing";
        if pkg != compiletime && !in_testing {
            return None;
        }
        let name = self.name_ref(info.name);
        Some(match (in_testing, name) {
            (true, "typeChecks" | "typeCheckErrors") => Intrinsic::TypeChecks,
            (true, _) => return None,
            (false, "erasedValue") => Intrinsic::ErasedValue,
            (false, "constValue") => Intrinsic::ConstValue,
            (false, "constValueOpt") => Intrinsic::ConstValueOpt,
            (false, "constValueTuple") => Intrinsic::ConstValueTuple,
            (false, "summonInline") => Intrinsic::SummonInline,
            (false, "summonFrom") => Intrinsic::SummonFrom,
            (false, "summonAll") => Intrinsic::SummonAll,
            (false, "error") => Intrinsic::Error,
            (false, "requireConst") => Intrinsic::RequireConst,
            (false, "codeOf") => Intrinsic::CodeOf,
            (false, "uninitialized") => Intrinsic::Uninitialized,
            (false, "deferred") => Intrinsic::Deferred,
            _ => return None,
        })
    }

    /// `LinkingInfo.linkTimePropertyBoolean` and its siblings, which Scala.js's linker answers.
    /// The JavaScript value of the native class `t`: its global or import, or the member of the
    /// native object it is declared in.
    fn js_class_value(&mut self, t: TypeId) -> Option<TExprId> {
        let t = self.dealias(t);
        let c = match self.types.get(t) {
            Type::Class(c, _) | Type::Ctor(c) => c,
            _ => return None,
        };
        let info = self.syms.class(c);
        match info.js_binding {
            Some(JsBinding::Import(i)) => return Some(self.prog.add(TExpr::JsImport(i))),
            Some(JsBinding::Global(n)) => return Some(self.prog.add(TExpr::JsGlobal(n, false))),
            Some(JsBinding::GlobalScope) => return None,
            None => {}
        }
        let (Owner::Class(o), JsKind::Native, name) = (info.owner, info.js, info.name) else { return None };
        if self.syms.class(o).kind != ClassKind::Object || self.syms.class(o).js != JsKind::Native {
            return None;
        }
        let module = self.prog.add(TExpr::Module(o));
        Some(self.prog.add(TExpr::JsSelect(module, name)))
    }

    fn is_link_time_property(&self, c: ClassId, pkg: PkgId, name: Name) -> bool {
        self.name_ref(name).starts_with("linkTimeProperty")
            && self.name_ref(self.syms.class(c).name) == "LinkingInfo"
            && self.name_ref(self.syms.pkg(pkg).name) == "scalajs"
            && self.syms.pkg(pkg).parent == Some(self.b.scala_pkg)
    }

    fn link_time_property(&mut self, name: &str) -> Option<LitVal> {
        Some(match name {
            "core/productionMode" => LitVal::Bool(self.inline.production),
            "core/esVersion" => LitVal::Int(12),
            "core/useECMAScript2015Semantics" => LitVal::Bool(true),
            "core/isWebAssembly" => LitVal::Bool(false),
            // A split build is ES modules, and so is a single file that imports or exports.
            "core/moduleKind" => LitVal::Int(if self.inline.es_modules || self.is_module() { 2 } else { 1 }),
            "core/linkerVersion" => LitVal::Str(self.interner.intern(SCALAJS_VERSION)),
            _ => return None,
        })
    }

    fn inline_body(&mut self, sym: SymId) -> Option<InlineBody> {
        let (file, def_id) = {
            let s = self.syms.sym(sym);
            (s.file, s.def)
        };
        // A library method's body comes with its class's conversion, whose pseudo file
        // gives it and what it makes their positions, the same whatever asked for it first.
        let (file, def_id) = match def_id {
            Some(d) => (file, d),
            None => {
                // A product's top-level definition is the package's, its `$package` object
                // converted as the package's pseudo file.
                if let Some(obj) = self.loaded.as_ref().and_then(|l| l.product_package_members.get(&sym).copied()) {
                    self.check_product_package(obj);
                } else {
                    let Owner::Class(c) = self.syms.sym(sym).owner else { return None };
                    if !self.is_library_member(sym) || !self.convert_library_class(c) {
                        return None;
                    }
                }
                let s = self.syms.sym(sym);
                (s.file, s.def?)
            }
        };
        let def = self.ast(file).def(def_id);
        let transparent = def.mods & mods::TRANSPARENT != 0;
        let (body, declared, clauses, ext_group) = match &def.kind {
            DefKind::Fun(f) => (f.body?, f.ret.is_some(), &f.clauses, f.ext_group),
            DefKind::Given(g) => (g.alias?, true, &g.clauses, 0),
            _ => return None,
        };
        let inline_params = clauses.iter().flat_map(|c| c.params.iter().map(|p| p.mods & mods::INLINE != 0)).collect();
        let defaults = clauses.iter().flat_map(|c| c.params.iter().map(|p| p.default)).collect();
        let span = self.ast(file).expr_span(body);
        Some(InlineBody { file, body, span, transparent, declared, inline_params, defaults, ext_group })
    }

    /// The expansion of a call of the inline method `call.sym` with its typed arguments, one
    /// per parameter in clause order, or `None` when the method has no body to expand.
    pub(super) fn expand_inline(
        &mut self,
        call: &MethodCall,
        sig: &MethodSig,
        subst: &Subst,
        args: &[TExprId],
        ret_ty: TypeId,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        if self.capturing() {
            self.inline.last_binds = false;
        }
        let expanded = self.expand_inline_call(call, sig, subst, args, ret_ty, span, expected);
        if self.capturing() {
            if let Some((e, _)) = expanded {
                self.capture_inline(call, subst, args, span, e);
            }
        }
        expanded
    }

    fn expand_inline_call(
        &mut self,
        call: &MethodCall,
        sig: &MethodSig,
        subst: &Subst,
        args: &[TExprId],
        ret_ty: TypeId,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<(TExprId, TypeId)> {
        self.open_receiver = false;
        let sym = call.sym;
        let checked = self.inline.entry.take().filter(|(s, _)| *s == sym).map(|(_, def)| def);
        #[cfg(debug_assertions)]
        if let Some(def) = &checked {
            debug_assert!(matches!(self.substitution_entry(sym), Ok(d) if Arc::ptr_eq(&d, def)), "the entry decision of a kept call moved");
        }
        if let Some(intrinsic) = self.intrinsic_of(sym) {
            let r = self.inline_intrinsic(intrinsic, sig, subst, args, ret_ty, span, expected);
            if matches!(intrinsic, Intrinsic::ConstValue | Intrinsic::ConstValueOpt | Intrinsic::ConstValueTuple | Intrinsic::SummonInline | Intrinsic::SummonAll) {
                self.mark_leaf(r.0);
            }
            return Some(r);
        }
        if let Some(literal) = self.written_array(sym, args, ret_ty) {
            return Some((literal, ret_ty));
        }
        if checked.is_none() {
            if let Some(ty) = self.failed_definition_call(call, subst, args, ret_ty) {
                return Some(self.failed_call(sym, ty));
            }
        }
        if self.inline.depth >= self.inline.max_depth {
            let msg = format!("Maximal number of successive inlines ({}) exceeded,\nMaybe this is caused by a recursive inline method?", self.inline.max_depth);
            self.error(span, msg);
            return Some((self.prog.add(TExpr::Unit), ERROR));
        }
        if checked.is_none() && self.syms.sym(sym).def.is_some() && self.is_abstract_member(sym) {
            let (name, owner) = (self.name_str(self.syms.sym(sym).name), self.syms.sym(sym).owner);
            let class = match owner {
                Owner::Class(c) => format!(" in {}", self.class_description(c)),
                _ => String::new(),
            };
            self.error(span, format!("Deferred inline method {}{} cannot be invoked", name, class));
            return Some((self.prog.add(TExpr::Unit), ERROR));
        }
        // The stored body of a program's method, or why its call takes the retype path (a
        // library's or the std's method, and what the entry decision retains, `substitution.rs`).
        let entry = match checked {
            Some(def) => Ok(def),
            None => self.substitution_entry(sym),
        };
        match entry {
            Err(super::substitution::Fallback::ForwardReference) => {
                let msg = self.inline.forward_refs[&sym].clone();
                self.error(span, msg);
                return Some((self.prog.add(TExpr::Unit), ret_ty));
            }
            // A record the entry decision found failed is the plain call's, never retyped.
            Err(super::substitution::Fallback::Failed) => {
                let ty = self.failed_definition_call(call, subst, args, ret_ty).unwrap_or(ERROR);
                return Some(self.failed_call(sym, ty));
            }
            _ => {}
        }
        let body = self.inline_body(sym)?;
        self.restrict_inline_call(sym, span);
        self.inline.expansions += 1;
        if body.file != self.env.file {
            self.inline_deps.entry(body.file).or_default().insert(self.env.file, ());
        }
        let prof = self.prof(Kind::Inline, span, About::Sym(sym));
        let (owner, def_span) = {
            let s = self.syms.sym(sym);
            (s.owner, s.span)
        };
        let part = self.part(Part::TypeArgs);
        let env = if owner == Owner::Local { self.env.clone() } else { self.env_at(body.file, owner, def_span.start) };
        let at_end = std::mem::replace(&mut self.inline.site_at_end, false);
        let call_node = self.body_node.filter(|n| n.file == self.env.file).map(|n| n.call);
        let site = InlineSite { callee: sym, file: self.env.file, span, body_file: body.file, body_span: body.span, at_end, call_node };
        let outer_site_env = self.inline.site_env.clone();
        let givens_mark = self.inline.outer_scopes.len();
        if self.inline.depth == 0 {
            self.inline.site_env = Some(Arc::new(self.env.clone()));
        } else {
            let locals = self.env_locals();
            self.inline.outer_locals.push(locals);
            let scopes = self.body_scopes();
            self.inline.outer_scopes.extend(scopes);
        }
        // The imports of the enclosing body are its scopes' now, which name its frames.
        let outer_imports = std::mem::take(&mut self.inline.body_imports);
        let diag_mark = self.diags.items.len();
        let tparam_mark = self.inline.tparams.len();
        let mut targs: u64 = 0xcbf2_9ce4_8422_2325;
        let mut note = |t: TypeId| targs = (targs ^ t.0 as u64).wrapping_mul(0x0100_0000_01b3);
        for &(tp, t) in subst {
            let t = self.zonk(t);
            let t = self.solve_if_var(t);
            note(t);
            self.bind_tparam(tp, t);
        }
        // The body of a method of a generic class names the class's parameters, which the
        // receiver's type fixes (`D extends Common[TC]` for a body over `TypeClass[_]`).
        for &(tp, t) in &call.owner_subst {
            let t = self.zonk(t);
            note(t);
            self.bind_tparam(tp, t);
        }
        if let Some(p) = prof {
            self.profile.set_targs(p, targs);
        }
        self.part_end(part);
        self.inline.depth += 1;
        if self.inline.sites.is_empty() {
            self.quote.begin_site(site.file, site.span.start);
        }
        self.inline.sites.push(site);
        // The body stands in its definition's scope, where the type variables of the patterns
        // around the call are not: its own patterns bind theirs afresh.
        let case_binders = std::mem::take(&mut self.case_binders);
        let result = self.with_env(env, |t| match &entry {
            Ok(def) if super::substitution::measures_copy() => {
                t.copy_for_measure(def, subst, &call.owner_subst);
                t.expand_in_scope(call, sig, subst, args, &body, ret_ty, span, expected)
            }
            Ok(def) => {
                super::substitution::counts::substituted();
                t.index_expanded_body(sym);
                t.inline.walks += 1;
                let r = t.expand_by_substitution(call, sig, subst, args, &body, ret_ty, span, def);
                t.inline.walks -= 1;
                r
            }
            Err(reason) => {
                super::substitution::counts::fell_back(*reason, t.inline.walks > 0);
                t.expand_in_scope(call, sig, subst, args, &body, ret_ty, span, expected)
            }
        });
        self.case_binders = case_binders;
        if self.records_expansions() {
            self.prog.note_expansion(result.0, Expansion { callee: sym });
        } else {
            self.prog.mark_expansion(result.0);
        }
        // A plain inline method's call ends a chain of `+` that it heads; a transparent one's
        // expansion stands where the call stood, to scalac's backend as to its typer.
        if !body.transparent {
            self.end_chain(result.0, result.1);
        }
        if let Some(ix) = self.index.as_mut() {
            ix.expansions.insert(result.0, sym);
        }
        self.inline.sites.pop();
        self.inline.depth -= 1;
        self.truncate_tparams(tparam_mark);
        self.inline.outer_scopes.truncate(givens_mark);
        self.inline.body_imports = outer_imports;
        if self.inline.depth == 0 {
            self.inline.site_env = outer_site_env;
        } else {
            self.inline.outer_locals.pop();
        }
        let part = self.part(Part::Relocate);
        self.relocate_diagnostics(diag_mark, &site);
        self.part_end(part);
        if let Some(p) = prof {
            self.profile.exit(p, Outcome::Found);
        }
        Some(result)
    }

    /// The plain call of a method whose definition failed the check, of type `ty`, which depends
    /// on the definition as an expansion does: an edit that fixes it types the caller again
    /// (`Worker::inline_deps`).
    fn failed_call(&mut self, sym: SymId, ty: TypeId) -> (TExprId, TypeId) {
        let def_file = self.syms.sym(sym).file;
        if def_file != self.env.file {
            self.inline_deps.entry(def_file).or_default().insert(self.env.file, ());
        }
        (self.prog.add(TExpr::Unit), ty)
    }

    fn expand_in_scope(
        &mut self,
        call: &MethodCall,
        sig: &MethodSig,
        subst: &Subst,
        args: &[TExprId],
        body: &InlineBody,
        ret_ty: TypeId,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let mut bound = self.bind_inline_call(call, sig, subst, args, body, ret_ty, span, false);
        let ret_ty = bound.ret_ty;
        let outer_return = self.return_to.take();
        let outer_returns = std::mem::take(&mut self.returns);
        let outer_partial = std::mem::replace(&mut self.partial_match, false);
        let (te, ty) = if body.transparent {
            let (te, ty) = self.type_expr(body.body, expected);
            // scalac folds the body to a constant of its literal type, which a transparent
            // method passes on to the call; a body that ends in an ascription (`(1: Any)`,
            // `(1: Int)`) has the type it was given, which the constant does not replace.
            let result = self.result_expr(body.body);
            let ascribed = matches!(self.cur_ast().expr(result), crate::ast::Expr::Typed(..)) || self.widened_constant(te);
            match self.fold_constant(te).filter(|_| !ascribed) {
                Some(v) => (te, self.types.lit(v)),
                None => (te, ty),
            }
        } else if !body.declared {
            let (te, ty) = self.type_expr(body.body, expected);
            match self.inferred_call_result(call, subst, args) {
                Some(inferred) => (te, inferred),
                None => (te, self.solve_inferred(ty)),
            }
        } else if let Some(own) = self.body_result_type(call, sig, subst, &bound) {
            (self.check_expr(body.body, own), ret_ty)
        } else {
            (self.check_expr(body.body, ret_ty), ret_ty)
        };
        if !body.transparent {
            self.mark_widened_expansion(te, ty);
        }
        self.return_to = outer_return;
        self.returns = outer_returns;
        self.partial_match = outer_partial;
        self.unbind_inline_call(&mut bound, te, ty)
    }

    /// The declared result a body is checked against where the call's differs from it in a
    /// parameter's path: the call's result is in the call's terms, where a parameter whose
    /// argument is no path stands as the argument's type (`Elem[This, n.type]` of `t(i)` is
    /// `Elem[T, Int]`), while the body names the parameter's local (`Elem[T, n.type]`). A result
    /// that also names the class's `this` keeps the call's.
    fn body_result_type(&mut self, call: &MethodCall, sig: &MethodSig, subst: &Subst, bound: &BoundCall) -> Option<TypeId> {
        if !self.types.has_paths(sig.ret) {
            return None;
        }
        let paths = self.inline.param_paths[bound.paths_mark..].to_vec();
        if !paths.iter().any(|&(p, _)| self.mentions_term(sig.ret, p)) {
            return None;
        }
        if let Owner::Class(c) = self.syms.sym(call.sym).owner {
            if self.mentions_this_of(sig.ret, c) {
                return None;
            }
        }
        let declared = self.types.subst(sig.ret, subst);
        Some(self.subst_paths(declared, &paths))
    }

    /// The receiver and the arguments of a call of an inline method bound for its body, as scalac
    /// binds them: the receiver first, then each argument in clause order, an ordinary one to a
    /// proxy val evaluated once unless it is stable or a constant, a by-name or `inline` one to
    /// its tree, which each use copies (`InlineState::args`). A missing argument with a default
    /// takes the default: typed here for the retype path, left for the expansion by substitution
    /// to bind from its copy of the record's (`defaults_later`, `bind_parameter`). The frame the
    /// body's names stand in is pushed; `unbind_inline_call` pops it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn bind_inline_call(
        &mut self,
        call: &MethodCall,
        sig: &MethodSig,
        subst: &Subst,
        args: &[TExprId],
        body: &InlineBody,
        ret_ty: TypeId,
        span: Span,
        defaults_later: bool,
    ) -> BoundCall {
        let sym = call.sym;
        let bind = self.part(Part::Bind);
        let arg_types = std::mem::take(&mut self.inline.arg_types);
        let tparams: Vec<(Name, TParamId)> = sig.tparams.iter().map(|&p| (self.syms.tparam(p).name, p)).collect();
        self.env.frames.push(Frame::Locals { names: Vec::new(), tparams, givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 });
        self.inline.body_frames.push(self.env.frames.len());
        let mut bound = BoundCall {
            stmts: Vec::new(),
            this_mark: self.inline.this.len(),
            this_paths_mark: self.inline.this_paths.len(),
            copies_mark: self.inline.copies.len(),
            this_arg: None,
            this_proxy: None,
            proxies: Vec::new(),
            givens: Vec::new(),
            result_names: None,
            outer_proxies: Vec::new(),
            paths_mark: 0,
            ret_ty,
            params: Vec::new(),
        };
        if let (Some(r), Owner::Class(c)) = (call.recv, self.syms.sym(sym).owner) {
            if self.is_stable(r) && !matches!(self.prog.expr(r), TExpr::This) {
                let recv_ty = self.inline.recv_types.get(&r).copied().unwrap_or_else(|| self.syms.this_type(c));
                let path = self.prefix_of(r, recv_ty);
                let object = matches!(self.types.get(path), Type::Class(k, _) if self.syms.class(k).kind == ClassKind::Object);
                if self.types.is_path(path) || object {
                    self.inline.this_paths.push((c, path));
                }
            }
            // `this` takes the receiver's own type, so that the members it names resolve as
            // they would on the receiver: an abstract inline member to its implementation.
            let this_ty = match self.inline.recv_types.remove(&r) {
                Some(t) if t != ERROR => t,
                _ => {
                    let declared = self.syms.this_type(c);
                    self.types.subst(declared, &call.owner_subst)
                }
            };
            // A stable receiver (an object, a val) is what the body names in the place of
            // `this`, as a stable argument is; any other is bound once, before the body.
            let proxy = match self.prog.expr(r) {
                TExpr::This => None,
                _ => {
                    let proxy = self.fresh_local("this", this_ty, span);
                    if !matches!(self.prog.expr(r), TExpr::Module(_)) {
                        self.mark_leaf(r);
                    }
                    if self.is_stable(r) {
                        self.inline.args.insert(proxy, InlineArg { expr: r, ty: this_ty, source: None });
                        bound.this_arg = Some(proxy);
                    } else {
                        bound.stmts.push((0, TStmt::Val(proxy, r)));
                        // `C.this` in the body's types is the proxy's path.
                        let path = self.types.mk(Type::Term(proxy));
                        self.inline.this_paths.push((c, path));
                    }
                    Some(proxy)
                }
            };
            bound.this_proxy = proxy;
            self.inline.this.push((c, proxy, this_ty));
        }
        // A call whose declared result type is the singleton of a parameter (`x.type` of
        // scala-library's `summon`) has, at the definition, the type of that parameter's argument:
        // an outer parameter as written gives the call that parameter's identity for the body.
        let mut index = 0;
        let mut receiver = None;
        // A macro sees the argument as written, folded no further than scalac folds a
        // constant expression.
        let is_macro = self.is_macro_body(body.file, body.body);
        for clause in &sig.clauses {
            for p in &clause.params {
                let dependent_result = self.types.has_paths(sig.ret) && self.mentions_term(sig.ret, p.sym);
                let is_inline = body.inline_params.get(index).copied().unwrap_or(false);
                let default = body.defaults.get(index).copied().flatten();
                let local_ty = self.sig_of(p.sym).ret;
                let local_ty = self.types.subst(local_ty, subst);
                let local_ty = self.zonk(local_ty);
                let local_ty = self.seen_from_receiver_paths(local_ty);
                let declared = local_ty;
                // A repeated parameter without arguments stands for the empty sequence.
                let arg = match args.get(index).copied() {
                    Some(a) => a,
                    None if p.repeated => {
                        let empty = self.prog.add(TExpr::SeqLit(crate::ast::ListRef::EMPTY));
                        self.prog.set_type(empty, local_ty);
                        empty
                    }
                    None => self.prog.add(TExpr::Unit),
                };
                index += 1;
                let later = defaults_later && p.has_default && default.is_some() && matches!(self.prog.expr(arg), TExpr::Unit);
                let te = match (p.by_name, self.prog.expr(arg), default) {
                    (true, TExpr::Lambda(ps, inner), _) if ps.is_empty() => inner,
                    (_, TExpr::Unit, Some(_)) if later => arg,
                    (_, TExpr::Unit, Some(d)) if p.has_default => self.check_expr(d, local_ty),
                    _ => arg,
                };
                if bound.result_names.is_none() && self.result_is_singleton_of(sig.ret, p.sym, declared) {
                    bound.result_names = self.parameter_named(te);
                }
                // The proxy takes the argument's own type, widened, as scalac's binding does
                // (`Inliner.paramBindingDef`): an `inline match` on the parameter then sees what was
                // passed. An argument of a type that is a bottom type after erasure, `Nothing` or
                // `Null` (`isBottomTypeAfterErasure`), binds at the parameter's type, and a `*:` chain
                // stays the chain it is spelled, whose erasure its spelling decides (`erasePair`).
                let arg_ty = arg_types.iter().rev().find(|(e, _)| *e == te).map(|&(_, t)| t);
                let local_ty = match arg_ty.filter(|_| !p.repeated) {
                    Some(t) => {
                        let solved = self.solve_in(t);
                        let t = match self.types.get(solved) {
                            Type::Class(c, _) if Some(c) == self.b.cons_tuple => solved,
                            _ => {
                                let normalized = self.normalize(solved);
                                self.widen_lit(normalized)
                            }
                        };
                        if t == ERROR || self.is_bottom_after_erasure(t) { local_ty } else { t }
                    }
                    None => local_ty,
                };
                // A stable argument of a method whose result type names the parameter
                // (`inst.type`) binds the proxy to the argument's own path, as scalac's binding
                // does, so that the body conforms to the result type in the call's terms.
                let local_ty = match dependent_result && !later {
                    true => self.argument_path(te).unwrap_or(local_ty),
                    false => local_ty,
                };
                let proxy = self.new_local(p.name, SymKind::Val, local_ty, span);
                if self.index.is_some() {
                    self.index_unrecord(proxy);
                }
                bound.proxies.push(proxy);
                // A recursive expansion under way keeps its own proxy for the parameter once
                // this one is done (`summonAsArray0[b](i + 1, arr)` after a nested one).
                let outer = self.inline.param_proxies.insert(p.sym, proxy);
                bound.outer_proxies.push((p.sym, outer));
                let param = BoundParam { sym: p.sym, name: p.name, proxy, arg: te, constant: None, declared, local_ty, by_name: p.by_name, inline: is_inline, repeated: p.repeated, is_using: clause.is_using, is_macro, later };
                bound.params.push(param);
                let at = bound.params.len() - 1;
                if !later {
                    self.bind_parameter(&mut bound, at, te);
                }
                self.bind_local(p.name, proxy);
                if receiver.is_none() && !clause.is_using {
                    receiver = Some(proxy);
                }
            }
        }
        if let (true, Some(receiver)) = (self.syms.sym(sym).is_extension, receiver) {
            let info = self.syms.sym(sym);
            self.ext_scope = Some(ExtScope { owner: info.owner, file: info.file, group: body.ext_group, receiver });
        }
        // A type that names a parameter (`x.type`) names the local standing for it.
        bound.paths_mark = self.inline.param_paths.len();
        let params: Vec<SymId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
        for (&p, &proxy) in params.iter().zip(&bound.proxies) {
            let path = self.types.mk(Type::Term(proxy));
            self.inline.param_paths.push((p, path));
        }
        // The result type is in the call's terms already; only its parameter paths move.
        if self.types.has_paths(bound.ret_ty) {
            let paths = self.inline.param_paths[bound.paths_mark..].to_vec();
            bound.ret_ty = self.subst_paths(bound.ret_ty, &paths);
        }
        self.part_end(bind);
        bound
    }

    /// Binds the parameter `at` of `bound` to its argument `te`: a constant argument to the
    /// constant, one the interpreter can evaluate to its value (marked as the output's, not the
    /// language's constant), a by-name, `inline`, constant or stable one standing for its tree at
    /// each use, any other to its proxy val; a using parameter also a given of its declared type.
    pub(super) fn bind_parameter(&mut self, bound: &mut BoundCall, at: usize, te: TExprId) {
        let BoundParam { name, proxy, declared, local_ty, by_name, inline, repeated, is_using, is_macro, .. } = bound.params[at];
        // An argument that folds to a constant is bound as that constant, as scalac's
        // inliner binds it, so that a condition on the parameter folds in the body. One
        // the interpreter can evaluate is bound to its value too, marked so that the
        // typing of the body does not read it as a constant of the language.
        let fold = self.part(Part::Fold);
        let constant = (!repeated).then(|| if is_macro { self.fold_structural(te) } else { self.fold_constant(te) }).flatten();
        let te = match constant {
            Some(v) => {
                let lit = self.literal_expr(v);
                self.prog.copy_span(te, lit);
                // The constant stands for the argument as its marks have it: ascribed, or a plain
                // inline call no operation folds over.
                if self.ascribed_root(te) {
                    self.prog.mark_widened(lit);
                }
                if self.opaque_operand(te) {
                    self.prog.mark_opaque(lit);
                }
                lit
            }
            None if !is_macro && !repeated => match self.fold_by_eval(te) {
                Some(v) => {
                    let lit = self.literal_expr(v);
                    self.prog.copy_span(te, lit);
                    self.inline.evaluated.insert(lit, ());
                    lit
                }
                None => te,
            },
            None => te,
        };
        self.part_end(fold);
        bound.params[at].constant = constant;
        bound.params[at].arg = te;
        self.mark_leaf(te);
        if by_name || inline || constant.is_some() || self.is_stable(te) {
            self.inline.args.insert(proxy, InlineArg { expr: te, ty: local_ty, source: None });
        } else {
            bound.stmts.push((at as u32 + 1, TStmt::Val(proxy, te)));
        }
        self.inline.param_bindings.insert(proxy, (te, declared));
        // A using parameter is a given of its declared type, as scalac's search in the
        // body found it at the definition: a local of that type standing for the value,
        // which keeps the argument's own type (`inner(summon[B])` binds a `C`).
        if is_using {
            let span = self.syms.sym(proxy).span;
            let given = self.new_local(name, SymKind::Val, declared, span);
            if self.index.is_some() {
                self.index_unrecord(given);
            }
            let value = if self.inline.args.contains_key(&proxy) { te } else { self.prog.add(TExpr::Local(proxy)) };
            self.inline.args.insert(given, InlineArg { expr: value, ty: local_ty, source: None });
            self.inline.param_bindings.insert(given, (te, declared));
            bound.givens.push(given);
            self.bind_given(given);
        }
    }

    /// Undoes what `bind_inline_call` bound once the body `te` of type `ty` is expanded, and
    /// gives the expansion: the body after the proxies' vals, in the order of their parameters.
    pub(super) fn unbind_inline_call(&mut self, bound: &mut BoundCall, te: TExprId, ty: TypeId) -> (TExprId, TypeId) {
        self.inline.body_frames.pop();
        self.env.frames.pop();
        self.inline.this.truncate(bound.this_mark);
        self.inline.this_paths.truncate(bound.this_paths_mark);
        if let Some(p) = bound.this_arg {
            self.inline.args.remove(&p);
        }
        self.inline.param_paths.truncate(bound.paths_mark);
        for &p in &bound.proxies {
            self.inline.args.remove(&p);
            self.inline.param_bindings.remove(&p);
        }
        for &g in &bound.givens {
            self.inline.args.remove(&g);
            self.inline.param_bindings.remove(&g);
        }
        self.inline.copies.truncate(bound.copies_mark);
        for &(param, outer) in &bound.outer_proxies {
            match outer {
                Some(o) => self.inline.param_proxies.insert(param, o),
                None => self.inline.param_proxies.remove(&param),
            };
        }
        let result = if bound.stmts.is_empty() {
            te
        } else {
            bound.stmts.sort_by_key(|&(at, _)| at);
            let stmts: Vec<TStmt> = bound.stmts.iter().map(|&(_, s)| s).collect();
            let l = self.prog.stmts.push_slice(&stmts);
            self.prog.add(TExpr::Block(l, te))
        };
        if self.capturing() {
            self.inline.last_binds = !bound.stmts.is_empty();
        }
        // Inside a body an inline call's result has the type the call has at the definition,
        // whatever parameter the body returned as written: the call is the boundary of the
        // parameter's identity (`pick(id(x))` ranks at `id`'s result type), unless that type
        // is the singleton of a parameter given an outer parameter as written.
        match bound.result_names {
            Some(outer) => self.inline.copies.push((result, outer)),
            None if !self.inline.param_bindings.is_empty() => {
                *self.expr_marks.entry(result).or_default() |= super::MARK_RETYPED;
            }
            None => {}
        }
        (result, ty)
    }

    /// Whether an inline method body is a splice: the method is a macro.
    pub(crate) fn is_macro_body(&self, file: FileId, body: ExprId) -> bool {
        let ast = self.ast(file);
        let mut e = body;
        loop {
            match ast.expr(e) {
                Expr::Splice(_) => return true,
                // A jar's macro is pickled ascribed its result, `(${ impl('e) }: String)`.
                Expr::Parens(inner) | Expr::Typed(inner, _) => e = inner,
                Expr::Block(stmts) => match ast.stmt_list(stmts) {
                    [crate::ast::Stmt::Expr(inner)] => e = *inner,
                    _ => return false,
                },
                _ => return false,
            }
        }
    }

    /// Diagnostics of the expanded body move to the call site, with a note of where the code
    /// was inlined from, as scalac's inline stack trace has it.
    fn relocate_diagnostics(&mut self, mark: usize, site: &InlineSite) {
        if self.diags.items.len() == mark {
            return;
        }
        let within = |span: Span| span.start >= site.body_span.start && span.end <= site.body_span.end;
        let moves = |d: &crate::source::Diagnostic| d.file == site.body_file && (within(d.span) || site.body_span == Span::default());
        if !self.diags.items[mark..].iter().any(moves) {
            return;
        }
        let origin = {
            let f = self.source(site.body_file);
            if f.text.is_empty() {
                format!("\n  inlined from {}", self.method_description(site.callee))
            } else {
                let (line, _, _) = crate::source::locate(&f.text, site.body_span.start as usize);
                format!("\n  inlined from {}:{}", f.path, line)
            }
        };
        // A diagnostic placed in a library's source is inlined from its place, and moved to the
        // call: the place of the call where the call is in a library body itself.
        let call_place = match site.call_node {
            Some(e) if self.diags.items[mark..].iter().any(|d| moves(d) && d.place.is_some()) => self.call_place(site.file, e),
            _ => None,
        };
        for d in &mut self.diags.items[mark..] {
            if moves(d) {
                if !d.at_expansion {
                    d.inlined_from.get_or_insert((d.file, d.span));
                }
                d.file = site.file;
                d.span = site.span;
                match d.place.take() {
                    Some(p) => {
                        d.msg.push_str(&format!("\n  inlined from {}", p.line_of()));
                        d.place = call_place.clone().map(Box::new);
                    }
                    None => d.msg.push_str(&origin),
                }
            }
        }
    }

    /// The singleton type of a stable argument: a val, a parameter, an object or a given that
    /// takes nothing. A given with type parameters or clauses has no path of its own, since
    /// its path would name the generic signature and not the instance the search chose.
    pub(super) fn argument_path(&mut self, te: TExprId) -> Option<TypeId> {
        let sym = match self.prog.expr(te) {
            TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) => Some(s),
            _ => None,
        };
        if let Some(s) = sym {
            let sig = self.sig_of(s);
            if !sig.tparams.is_empty() || !sig.clauses.is_empty() {
                return None;
            }
        }
        self.path_of(te).filter(|&p| self.types.is_path(p))
    }

    /// The expression a substituted parameter stands for, copied for this use.
    /// The scopes that the body under expansion opens around the point being typed, as the
    /// givens each defines and the imports it holds, outermost first: its own frames, not its
    /// definition's nor its using parameters', whose proxies scalac does not make givens.
    fn body_scopes(&self) -> Vec<BodyScope> {
        let Some(&base) = self.inline.body_frames.last() else { return Vec::new() };
        let mut out = Vec::new();
        for (at, frame) in self.env.frames.iter().enumerate().skip(base) {
            let givens = match frame {
                Frame::Locals { givens, .. } => givens.clone(),
                _ => Vec::new(),
            };
            let imports: Vec<ResolvedImport> = self.inline.body_imports.iter().filter(|&&(f, _)| f == at).map(|&(_, i)| i).collect();
            if !givens.is_empty() || !imports.is_empty() {
                out.push(BodyScope { givens, imports });
            }
        }
        out
    }

    /// The locals the frames of the current scope bind.
    fn env_locals(&self) -> Vec<SymId> {
        let mut out = Vec::new();
        for frame in &self.env.frames {
            if let Frame::Locals { names, givens, .. } = frame {
                out.extend(names.iter().map(|&(_, s)| s));
                out.extend(givens.iter().copied());
            }
        }
        out
    }

    /// Whether every value of `t` is `null` or none, as erased (`Types.isBottomTypeAfterErasure`):
    /// `Nothing` and `Null`.
    fn is_bottom_after_erasure(&mut self, t: TypeId) -> bool {
        let t = self.dealias(t);
        t == NOTHING || matches!(self.types.get(t), Type::Class(c, _) if c == self.b.null)
    }

    pub fn inline_arg(&mut self, s: SymId) -> Option<(TExprId, TypeId)> {
        if self.inline.args.is_empty() {
            return None;
        }
        let arg = *self.inline.args.get(&s)?;
        let part = self.body_part(Part::Copy);
        let copied = self.copy_expr(arg.expr);
        self.part_end(part);
        self.inline.copies.push((copied, s));
        // The copy stands where the proxy stood, so it carries the proxy's type: a mirror
        // passed for a `Mirror.ProductOf[T]` keeps the refinement its value's own type lacks.
        if self.types.has_paths(arg.ty) || matches!(self.types.get(arg.ty), Type::Refined(..)) {
            self.prog.set_type(copied, arg.ty);
        }
        Some((copied, arg.ty))
    }

    /// The declared type of the parameter that `te` stands for as written: a parameter's proxy
    /// or the copy of a substituted argument, not a cast or an ascription of one, which the
    /// typer hands back as the same node marked `MARK_RETYPED`. scalac types an inline body
    /// once, at the definition, so an argument, a scrutinee or a receiver that names a
    /// parameter is resolved at that type; the proxy itself carries the argument's own type
    /// for `inline match`, for the type of a transparent expansion and for the class in which
    /// a deferred inline member's implementation is found (`parameter_member`).
    pub(super) fn proxy_declared(&self, te: TExprId) -> Option<TypeId> {
        let proxy = self.parameter_named(te)?;
        self.inline.param_bindings.get(&proxy).map(|&(_, declared)| declared)
    }

    /// The parameter (its proxy, or the given standing for a using parameter) that `te` names
    /// as written.
    pub(super) fn parameter_named(&self, te: TExprId) -> Option<SymId> {
        if self.inline.param_bindings.is_empty() {
            return None;
        }
        // A copy first: a substituted argument may be the outer expansion's proxy itself
        // (`outer(x: B) = inner(x)`), which stands for the inner parameter here.
        let proxy = match self.inline.copies.iter().rev().find(|&&(copy, _)| copy == te) {
            Some(&(_, proxy)) => proxy,
            None => match self.prog.expr(te) {
                TExpr::Local(s) if self.inline.param_bindings.contains_key(&s) => s,
                _ => return None,
            },
        };
        if self.expr_marks.get(&te).map_or(false, |&m| m & super::MARK_RETYPED != 0) {
            return None;
        }
        Some(proxy)
    }

    /// The member `name` of a parameter named as written, with its owner's type: resolved on
    /// the parameter's declared type, as scalac resolved it at the definition, and where that
    /// is a deferred inline member, its implementation in the class of the argument's own type
    /// `own`, which is how scalac dispatches the call it inlines.
    pub(super) fn parameter_member(&mut self, recv: TExprId, own: TypeId, name: Name) -> Option<(SymId, TypeId)> {
        let declared = self.proxy_declared(recv)?;
        if declared == own {
            return None;
        }
        let (member, owner_ty) = self.find_member(declared, name)?;
        if self.is_inline_callee(member) && self.is_abstract_member(member) {
            if let Some((implementation, owner)) = self.find_member(own, name) {
                if !self.is_abstract_member(implementation) {
                    return Some((implementation, owner));
                }
            }
        }
        Some((member, owner_ty))
    }

    /// The implementation, in the class of the argument's own type `own`, of the deferred inline
    /// member `m` that a parameter's member resolved to on the declared type, with the owner's
    /// type: an alternative of an overloaded name, which `parameter_member` cannot dispatch
    /// before the arguments choose it.
    pub(super) fn deferred_implementation(&mut self, recv: TExprId, own: TypeId, m: SymId) -> Option<(SymId, TypeId)> {
        if !self.is_inline_callee(m) || !self.is_abstract_member(m) {
            return None;
        }
        self.proxy_declared(recv)?;
        let mut classes = Vec::new();
        self.own_classes(own, &mut classes);
        let implementation = classes.into_iter().find_map(|c| {
            let bases = self.syms.class(c).base_types.clone();
            self.implementation_of(c, &bases, m).filter(|&i| i != m)
        })?;
        let Owner::Class(owner) = self.syms.sym(implementation).owner else { return None };
        let owner_ty = self.base_type(own, owner)?;
        Some((implementation, owner_ty))
    }

    /// The classes an argument's own type is an instance of: every part of an intersection,
    /// a refinement's parent, and for any other type what `class_of` names (a class, a type
    /// parameter's upper bound, a union's common class, a singleton's underlying type).
    pub(super) fn own_classes(&mut self, t: TypeId, out: &mut Vec<ClassId>) {
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Inter(a, b) => {
                self.own_classes(a, out);
                self.own_classes(b, out);
            }
            Type::Refined(parent, _) => self.own_classes(parent, out),
            _ => {
                let under = self.dealias(t);
                if under != t {
                    self.own_classes(under, out);
                } else if let Some(c) = self.class_of(t) {
                    out.push(c);
                }
            }
        }
    }

    /// Whether a declared result type is exactly the singleton of the parameter `p` (`x.type`,
    /// an alias of it, or its intersection with types the parameter's declared type `declared`
    /// conforms to, `x.type & B` over `x: B`), not a type that mentions it (`List[x.type]`,
    /// `x.Out`, `x.type & C` below the declared type), which is the call's own.
    fn result_is_singleton_of(&mut self, ret: TypeId, p: SymId, declared: TypeId) -> bool {
        if !self.types.has_paths(ret) {
            return false;
        }
        let mut parts = Vec::new();
        self.intersection_parts(ret, &mut parts);
        let mut singleton = false;
        let mark = self.snapshot();
        let holds = parts.iter().all(|&part| {
            if matches!(self.types.get(part), Type::Term(x) if x == p) {
                singleton = true;
                true
            } else {
                self.is_sub(declared, part)
            }
        });
        self.rollback(mark);
        holds && singleton
    }

    /// The parts of an intersection, each dealiased (a singleton kept as it is).
    fn intersection_parts(&mut self, t: TypeId, out: &mut Vec<TypeId>) {
        let mut t = self.deref(t);
        loop {
            match self.types.get(t) {
                Type::Inter(a, b) => {
                    self.intersection_parts(a, out);
                    self.intersection_parts(b, out);
                    return;
                }
                Type::Alias(..) => {
                    let under = self.deref_alias(t);
                    if under == t {
                        break;
                    }
                    t = under;
                }
                _ => break,
            }
        }
        out.push(t);
    }

    /// The argument's own type behind a parameter named as written: what its proxy carries.
    pub(super) fn proxy_own(&mut self, te: TExprId) -> Option<TypeId> {
        let proxy = self.parameter_named(te)?;
        match self.inline.args.get(&proxy) {
            Some(arg) => Some(arg.ty),
            None => Some(self.sig_of(proxy).ret),
        }
    }

    /// Whether the path of the term `s` occurs in `t`.
    pub fn mentions_term(&self, t: TypeId, s: SymId) -> bool {
        if !self.types.has_paths(t) {
            return false;
        }
        match self.types.get(t) {
            Type::Term(x) => x == s,
            Type::Select(p, _) | Type::Member(p, _) | Type::Lambda(_, p) => self.mentions_term(p, s),
            Type::Poly(ps, p) => self.mentions_term(p, s) || self.types.poly_bounds(ps).to_vec().into_iter().any(|b| self.mentions_term(b, s)),
            Type::AppMember(m, args) => self.mentions_term(m, s) || self.types.items(args).iter().any(|&a| self.mentions_term(a, s)),
            Type::Class(_, args) | Type::AppParam(_, args) | Type::AppVar(_, args) | Type::Alias(_, args) => {
                self.types.items(args).iter().any(|&a| self.mentions_term(a, s))
            }
            Type::Union(a, b) | Type::Inter(a, b) => self.mentions_term(a, s) || self.mentions_term(b, s),
            Type::Refined(parent, r) => {
                self.mentions_term(parent, s)
                    || match self.types.refinement(r) {
                        Refinement::Alias(_, x) => self.mentions_term(x, s),
                        Refinement::Bounds(_, lo, hi) => self.mentions_term(lo, s) || self.mentions_term(hi, s),
                        Refinement::Val(_, _, x) => self.mentions_term(x, s),
                        Refinement::Term(..) => false,
                    }
            }
            Type::Match(scrut, _) => self.mentions_term(scrut, s),
            _ => false,
        }
    }

    /// The type of `this` in the expansion under way, for a class the expanded method's owner
    /// is or derives from: the receiver's type, through which a type member the class leaves
    /// abstract resolves to what the receiver's class defines.
    pub fn inline_this_of(&self, c: ClassId) -> Option<TypeId> {
        self.inline_this(c).map(|(_, t)| t).filter(|&t| t != ERROR)
    }

    pub fn inline_tparam(&self, p: TParamId) -> Option<TypeId> {
        let t = self.inline.tparams.iter().rev().find(|(q, _)| *q == p).map(|&(_, t)| t);
        if t.is_some() {
            self.inline.tparam_reads.set(self.inline.tparam_reads.get().wrapping_add(1));
        }
        t
    }

    /// Whether the typer records the expansions and their leaves, which a build of JavaScript
    /// asks for (`InlineState::outlines`).
    #[inline]
    pub fn records_expansions(&self) -> bool {
        self.inline.outlines
    }

    pub fn mark_leaf(&mut self, e: TExprId) {
        if self.records_expansions() {
            self.prog.note_leaf(e);
        }
    }

    /// The branch a condition on the call site's constants selected, when it is a literal: a
    /// value of the site, as the constant it was selected by. `read_leaf` tells whether the
    /// condition read a leaf: the count of leaves the folding met moved (`leaf_reads`), or,
    /// for a condition the interpreter or its type folded, a leaf among its first nodes.
    pub fn mark_folded_leaf(&mut self, branch: TExprId, read_leaf: impl FnOnce(&Self) -> bool) {
        let literal = matches!(self.prog.expr(branch), TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_));
        if literal && self.records_expansions() && read_leaf(self) {
            self.prog.note_leaf(branch);
        }
    }

    pub fn leaf_near(&self, cond: TExprId) -> bool {
        self.prog.descendants(cond).take(32).any(|e| self.prog.is_leaf(e))
    }

    /// Marks the test as a leaf of the expansion under way when the type it tests read a type
    /// argument of the call since `reads`.
    pub fn mark_leaf_test(&mut self, test: TestId, reads: u32) {
        if self.inline.depth > 0 && self.inline.tparam_reads.get() != reads && self.records_expansions() {
            self.prog.leaf_tests.insert(test, ());
        }
    }

    pub(super) fn bind_tparam(&mut self, p: TParamId, t: TypeId) {
        self.inline.tparams.push((p, t));
        self.inline.tparams_gen += 1;
    }

    pub(super) fn truncate_tparams(&mut self, mark: usize) {
        if self.inline.tparams.len() > mark {
            self.inline.tparams.truncate(mark);
            self.inline.tparams_gen += 1;
            if mark == 0 {
                self.inline.tparams_subst = None;
            }
        }
    }

    /// The bound type parameters as one substitution: the innermost expansion's binding of a
    /// parameter, as `inline_tparam` answers, since a method expanded inside its own expansion
    /// (`derivedMirror[s]` under `derivedMirror[A]`) binds the same parameter again. Built once
    /// per state of the stack, which a resolved type of a library body asks for at every node.
    fn inline_tparams_subst(&mut self) -> Arc<Subst> {
        let gen = self.inline.tparams_gen;
        if let Some((built, subst)) = &self.inline.tparams_subst {
            if *built == gen {
                return subst.clone();
            }
        }
        let mut subst: Subst = Vec::with_capacity(self.inline.tparams.len());
        for &(p, t) in self.inline.tparams.iter().rev() {
            if !subst.iter().any(|&(q, _)| q == p) {
                subst.push((p, t));
            }
        }
        let subst = Arc::new(subst);
        self.inline.tparams_subst = Some((gen, subst.clone()));
        subst
    }

    /// A type the loader resolved, with the type parameters of the expansions under way bound
    /// and the paths of their parameters redirected to the locals standing for them.
    pub fn subst_inline_tparams(&mut self, t: TypeId) -> TypeId {
        let mut r = t;
        if !self.inline.tparams.is_empty() {
            let subst = self.inline_tparams_subst();
            r = self.types.subst(r, &subst);
            if self.types.is_reducible(r) {
                r = self.specialise_aliases(r, &subst);
            }
            if r != t {
                self.inline.tparam_reads.set(self.inline.tparam_reads.get().wrapping_add(1));
            }
        }
        if !self.inline.param_paths.is_empty() && self.types.has_paths(r) {
            let paths = std::mem::take(&mut self.inline.param_paths);
            r = self.subst_paths(r, &paths);
            self.inline.param_paths = paths;
        }
        self.seen_from_receiver_paths(r)
    }

    /// `C.this` in a type of an inline body, seen from the path the receiver of the expansion
    /// is, as the caller saw the member's signature.
    pub fn seen_from_receiver_paths(&mut self, t: TypeId) -> TypeId {
        if self.inline.this_paths.is_empty() || !self.types.has_paths(t) {
            return t;
        }
        let mut r = t;
        let paths = std::mem::take(&mut self.inline.this_paths);
        for &(c, p) in paths.iter().rev() {
            r = self.as_seen_from(r, p, c);
        }
        self.inline.this_paths = paths;
        r
    }

    /// `scala.arrayOf(head, rest*)` of the std with `rest` written out: the array literal of
    /// the elements, which every target makes in place, of the kind the type names.
    pub(super) fn written_array(&mut self, callee: SymId, args: &[TExprId], ty: TypeId) -> Option<TExprId> {
        let info = self.syms.sym(callee);
        if info.name != names::ARRAY_OF || info.owner != Owner::Package(self.b.scala_pkg) || !self.source(info.file).is_std {
            return None;
        }
        let (&head, &rest) = (args.first()?, args.get(1)?);
        let TExpr::SeqLit(items) = self.prog.expr(rest) else { return None };
        let mut elements = vec![head];
        elements.extend_from_slice(self.prog.expr_list(items));
        let l = self.prog.list(&elements);
        let literal = self.prog.add(TExpr::ArrayLit(l));
        self.prog.set_type(literal, ty);
        Some(literal)
    }

    /// What a reference by symbol names: the proxy of a parameter of the expansion under way,
    /// a local, or a member seen from inside its class.
    pub fn sym_ref_term(&mut self, s: SymId) -> super::resolve::TermRef {
        use super::resolve::TermRef;
        let s = self.inline.param_proxies.get(&s).copied().unwrap_or(s);
        match self.syms.sym(s).owner {
            Owner::Local => TermRef::Local(s),
            Owner::Class(c) if self.syms.class(c).kind == ClassKind::Object => TermRef::ModuleMember(c, s),
            Owner::Class(c) => TermRef::This(self.holder_of(c), s),
            Owner::Package(_) => TermRef::Global(s),
        }
    }

    /// What `this` of the class `c` is inside the expansions under way: the receiver of the
    /// expansion of a method of `c` or of a class below it (`join` of `Common`, named in a body
    /// of `Derivation`, is the receiver's).
    pub fn inline_this(&self, c: ClassId) -> Option<(Option<SymId>, TypeId)> {
        self.inline
            .this
            .iter()
            .rev()
            .find(|(k, _, _)| *k == c || self.syms.class(*k).base_types.iter().any(|&(b, _)| b == c))
            .map(|&(_, s, t)| (s, t))
    }

    pub fn source_text(&self, file: FileId, span: Span) -> String {
        let text = &self.source(file).text;
        match text.get(span.start as usize..span.end as usize) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => "<expression>".to_string(),
        }
    }

    // ---- copying typed trees ----

    /// A fresh copy of a typed expression, so that a substituted argument is evaluated at each
    /// use and no node is shared between two places of the tree.
    pub fn copy_expr(&mut self, e: TExprId) -> TExprId {
        // A typed tree copied outside every expansion has its pending plain calls expanded first,
        // so that no copy keeps a call of an inline method; inside one,
        // the expansion's arguments were (`expand_pending_in`).
        if self.inline.depth == 0 && self.attempts.pending_len() != 0 && !self.attempts.copying {
            self.attempts.copying = true;
            self.expand_pending_in(&[e]);
            self.attempts.copying = false;
        }
        let ty = self.prog.expr_types.get(e.0).unwrap_or(NO_TYPE);
        let mut carries_chain = false;
        let copied = match self.prog.expr(e) {
            TExpr::Int(_)
            | TExpr::Long(_)
            | TExpr::Double(_)
            | TExpr::Bool(_)
            | TExpr::Char(_)
            | TExpr::Str(_)
            | TExpr::Unit
            | TExpr::Local(_)
            | TExpr::This
            | TExpr::Super(_)
            | TExpr::Static(_)
            | TExpr::Module(_)
            | TExpr::ClassOf(_)
            | TExpr::JsImport(_)
            | TExpr::JsGlobal(..)
            | TExpr::Null => self.prog.expr(e),
            TExpr::Field(r, s) => TExpr::Field(self.copy_expr(r), s),
            TExpr::CallStatic(s, args) => TExpr::CallStatic(s, self.copy_list(args)),
            TExpr::CallMethod(r, s, args) => {
                let r = self.copy_expr(r);
                TExpr::CallMethod(r, s, self.copy_list(args))
            }
            TExpr::CallClosure(f, args) => {
                let f = self.copy_expr(f);
                TExpr::CallClosure(f, self.copy_list(args))
            }
            TExpr::New(c, args) => TExpr::New(c, self.copy_list(args)),
            TExpr::NewVia(s, args) => TExpr::NewVia(s, self.copy_list(args)),
            TExpr::Lambda(params, body) => TExpr::Lambda(params, self.copy_expr(body)),
            TExpr::If(c, t, els) => {
                let c = self.copy_expr(c);
                let t = self.copy_expr(t);
                carries_chain = true;
                TExpr::If(c, t, els.map(|x| self.copy_expr(x)))
            }
            TExpr::While(c, b) => {
                let c = self.copy_expr(c);
                TExpr::While(c, self.copy_expr(b))
            }
            TExpr::Block(stmts, res) => {
                let items: Vec<TStmt> = self.prog.stmts[stmts.range()].to_vec();
                let copied: Vec<TStmt> = items
                    .into_iter()
                    .map(|s| match s {
                        TStmt::Expr(x) => TStmt::Expr(self.copy_expr(x)),
                        TStmt::Val(v, x) => TStmt::Val(v, self.copy_expr(x)),
                        TStmt::Pat(p, x) => TStmt::Pat(p, self.copy_expr(x)),
                        TStmt::Fun(f) => TStmt::Fun(f),
                    })
                    .collect();
                let l = self.prog.stmts.push_slice(&copied);
                carries_chain = true;
                TExpr::Block(l, self.copy_expr(res))
            }
            TExpr::Assign(a, b) => {
                let a = self.copy_expr(a);
                TExpr::Assign(a, self.copy_expr(b))
            }
            TExpr::Match(scrut, cases) => {
                let scrut = self.copy_expr(scrut);
                TExpr::Match(scrut, self.copy_cases(cases))
            }
            TExpr::Prim(op, a, b) => {
                let a = self.copy_expr(a);
                TExpr::Prim(op, a, self.copy_expr(b))
            }
            TExpr::Unary(op, a) => TExpr::Unary(op, self.copy_expr(a)),
            TExpr::StrConcat(l) => {
                carries_chain = true;
                TExpr::StrConcat(self.copy_list(l))
            }
            TExpr::ToStr(a, k) => TExpr::ToStr(self.copy_expr(a), k),
            TExpr::Js(s, args) => TExpr::Js(s, self.copy_list(args)),
            TExpr::TypeTest(a, t) => TExpr::TypeTest(self.copy_expr(a), t),
            TExpr::Cast(a, op, t) => TExpr::Cast(self.copy_expr(a), op, t),
            TExpr::SeqLit(l) => TExpr::SeqLit(self.copy_list(l)),
            TExpr::ArrayLit(l) => TExpr::ArrayLit(self.copy_list(l)),
            TExpr::Index(a, i) => TExpr::Index(self.copy_expr(a), i),
            TExpr::JsSelect(a, n) => TExpr::JsSelect(self.copy_expr(a), n),
            TExpr::ObjLit(l) => TExpr::ObjLit(self.copy_list(l)),
            TExpr::Spread(a) => TExpr::Spread(self.copy_expr(a)),
            TExpr::Return(a) => TExpr::Return(self.copy_expr(a)),
            TExpr::Throw(a, wraps) => TExpr::Throw(self.copy_expr(a), wraps),
            TExpr::Splice(a) => TExpr::Splice(self.copy_expr(a)),
            TExpr::Try(i) => {
                let (body, cases, finalizer, wraps) = {
                    let t = &self.prog.tries[i as usize];
                    (t.body, t.cases, t.finalizer, t.wraps)
                };
                let body = self.copy_expr(body);
                let cases = self.copy_cases(cases);
                let finalizer = finalizer.map(|f| self.copy_expr(f));
                self.prog.tries.push(TTry { body, cases, finalizer, wraps });
                TExpr::Try(self.prog.tries.len() as u32 - 1)
            }
        };
        let id = self.prog.add(copied);
        if self.profile.on {
            self.profile.inline.copied_nodes += 1;
        }
        if ty != NO_TYPE {
            self.prog.set_type(id, ty);
        }
        self.prog.copy_span(e, id);
        if self.inline.evaluated.contains_key(&e) {
            self.inline.evaluated.insert(id, ());
        }
        if self.prog.is_leaf(e) {
            self.prog.note_leaf(id);
        }
        if self.prog.is_expansion(e) {
            match self.prog.expansions.get(&e) {
                Some(&x) => self.prog.note_expansion(id, x),
                None => self.prog.mark_expansion(id),
            }
        }
        if carries_chain {
            self.prog.copy_chain_marks(e, id);
        }
        if self.prog.is_widened(e) {
            self.prog.mark_widened(id);
        }
        if self.prog.is_opaque(e) {
            self.prog.mark_opaque(id);
        }
        if self.prog.is_spread(e) {
            self.prog.mark_spread(id);
        }
        // A pending call's copy in an expansion is pending in its turn (`pend_copy`).
        if self.inline.depth > 0 && self.attempts.pending_len() != 0 && self.is_pending_call(e) {
            self.pend_copy(e, id);
        }
        // An erased value's copy is one too (`Erasure.checkNotErased` of every copy kept).
        if !self.erased_values.is_empty() {
            if let Some(&(_, file, at)) = self.erased_values.iter().find(|m| m.0 == e) {
                self.erased_values.push((id, file, at));
            }
        }
        if self.interpolations.contains_key(&e) {
            self.interpolations.insert(id, ());
        }
        if self.soft_exprs.contains_key(&e) {
            self.soft_exprs.insert(id, ());
        }
        if self.capturing() {
            self.capture_copy(e, id);
        }
        id
    }

    fn copy_list(&mut self, l: ListRef) -> ListRef {
        let items: Vec<TExprId> = self.prog.expr_list(l).to_vec();
        let copied: Vec<TExprId> = items.into_iter().map(|x| self.copy_expr(x)).collect();
        self.prog.list(&copied)
    }

    fn copy_cases(&mut self, l: ListRef) -> ListRef {
        let items: Vec<TCase> = self.prog.cases[l.range()].to_vec();
        let copied: Vec<TCase> = items
            .into_iter()
            .map(|c| TCase { pat: c.pat, guard: c.guard.map(|g| self.copy_expr(g)), body: self.copy_expr(c.body) })
            .collect();
        self.prog.cases.push_slice(&copied)
    }

    // ---- constant folding ----

    /// The constant a typed expression evaluates to: one made of literals is folded here, one
    /// made of literals and calls of the standard library is run by the interpreter.
    /// A constant as scalac's constant folder sees one: a literal, a value of a literal type,
    /// a primitive operation over constants and the concatenation or comparison of constant
    /// strings. What the interpreter evaluated an argument to is not one, unless the dialect
    /// says so.
    pub fn fold_constant(&mut self, e: TExprId) -> Option<LitVal> {
        self.fold_tree(e, false).or_else(|| if self.dialect.interpreted_constants { self.fold_by_eval(e) } else { None })
    }

    /// The constant `e` is as it has been typed: scalac's, without typing the body of a val on
    /// the way and without a trace in the expansion under way, so that asking changes nothing.
    pub fn fold_constant_as_typed(&mut self, e: TExprId) -> Option<LitVal> {
        let reads = self.inline.leaf_reads;
        let outer = std::mem::replace(&mut self.inline.as_typed, true);
        let v = self.fold_tree(e, false);
        self.inline.as_typed = outer;
        self.inline.leaf_reads = reads;
        v
    }

    /// The constant `e` is with the pending calls in its tree read as their types, as scalac's
    /// typer folds them (`M.one + 0` over `inline def one: 1` is 1), none expanded.
    pub(super) fn fold_by_pending_types(&mut self, e: TExprId) -> Option<LitVal> {
        let outer = std::mem::replace(&mut self.attempts.by_pending_types, true);
        let v = self.fold_tree(e, false);
        self.attempts.by_pending_types = outer;
        v
    }

    /// Whether the constant `e` is one scalac types at a type that is no literal type: an
    /// ascription's (`Program::widened_bits`) or a plain inline call's (`Program::opaque_bits`),
    /// a block ending in one, or an operation over a plain inline call, which scalac's typer
    /// keeps as a call. An operation over an ascription is not: dotty folds it (`ConstFold`'s
    /// `ConstantTree` looks through `Typed`), so `(2: Int) + 0` is the constant 2 where `idx + 0`
    /// of an `inline def idx: Int` is none.
    #[inline(always)]
    pub(super) fn widened_constant(&self, e: TExprId) -> bool {
        self.prog.is_widened(e) || self.prog.is_opaque(e) || matches!(self.prog.expr(e), TExpr::Block(..) | TExpr::Prim(..) | TExpr::Unary(..) | TExpr::ToStr(..)) && self.widened_inside(e)
    }

    #[inline(never)]
    fn widened_inside(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::Block(_, res) => self.widened_constant(res),
            TExpr::Unary(_, a) | TExpr::ToStr(a, _) => self.opaque_operand(a),
            TExpr::Prim(_, a, b) => self.opaque_operand(a) || self.opaque_operand(b),
            _ => false,
        }
    }

    /// Whether an ascription widened `e`, or the result of the block `e` is.
    pub(super) fn ascribed_root(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::Block(_, res) => self.ascribed_root(res),
            _ => self.prog.is_widened(e),
        }
    }

    /// Whether the constant `e` is none to the check of an `inline val` in an inline method's
    /// body, which scalac's `InlineVals` makes where the body expands, after the inlining phase
    /// has expanded the plain inline calls: the type of the value as typed there is no literal
    /// type where an ascription or a plain inline call is its root, through a block
    /// (`inline val x = f1()`), where an operation over either folds (`f1() + 1L` of an
    /// `inline def f1(): Long = 1L` is the constant 2).
    pub(super) fn expanded_widening(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::Block(_, res) => self.expanded_widening(res),
            _ => self.prog.is_widened(e) || self.prog.is_opaque(e),
        }
    }

    /// An operand no operation over folds: a plain inline call, an operation over one, an
    /// ascription of either and a block ending in one (`ConstantTree` looks through `Typed`, and
    /// a block is a constant by its type).
    pub(super) fn opaque_operand(&self, e: TExprId) -> bool {
        if self.prog.is_opaque(e) {
            return true;
        }
        match self.prog.expr(e) {
            TExpr::Unary(_, a) | TExpr::ToStr(a, _) => self.opaque_operand(a),
            TExpr::Prim(_, a, b) => self.opaque_operand(a) || self.opaque_operand(b),
            TExpr::Block(_, res) => self.opaque_operand(res),
            _ => false,
        }
    }

    /// A plain inline method's expansion `e`, of type `ty`, at a declared type that is no literal
    /// type: scalac types the call at it, so a constant it folds to is widened. Only a node that
    /// can fold to one is marked.
    #[inline]
    pub(super) fn mark_widened_expansion(&mut self, e: TExprId, ty: TypeId) {
        if !matches!(self.prog.expr(e), TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Char(_) | TExpr::Bool(_) | TExpr::Str(_) | TExpr::Unary(..) | TExpr::Prim(..) | TExpr::ToStr(..) | TExpr::StrConcat(_) | TExpr::Block(..)) {
            return;
        }
        let literal = match self.types.get(ty) {
            Type::Lit(_) => true,
            Type::Class(..) => false,
            _ => self.fold_type(ty).is_some(),
        };
        if !literal {
            self.prog.mark_opaque(e);
        }
    }

    /// The constant a macro sees an argument as: scalac's, and the field of a case class or
    /// tuple built in place, which its inliner reduces.
    fn fold_structural(&mut self, e: TExprId) -> Option<LitVal> {
        self.fold_tree(e, true)
    }

    fn fold_tree(&mut self, e: TExprId, projections: bool) -> Option<LitVal> {
        use LitVal::*;
        if self.prog.is_leaf(e) {
            self.inline.leaf_reads += 1;
        }
        if self.attempts.by_pending_types {
            if let Some(t) = self.pending_type(e) {
                return self.fold_type(t);
            }
        }
        let literal = |t: &Self, v: LitVal| (t.dialect.interpreted_constants || !t.inline.evaluated.contains_key(&e)).then_some(v);
        Some(match self.prog.expr(e) {
            TExpr::Int(v) => return literal(self, Int(v)),
            TExpr::Long(v) => return literal(self, Long(v)),
            TExpr::Double(v) => return literal(self, Double(v.to_bits())),
            TExpr::Bool(v) => return literal(self, Bool(v)),
            TExpr::Char(v) => return literal(self, Char(v)),
            TExpr::Str(s) => {
                let v = Str(self.interner.intern(&self.prog.strings[s.idx()]));
                return literal(self, v);
            }
            // The bindings an inline match reduced to leave a block of pure values around the
            // constant, as scalac's reduction does.
            TExpr::Block(stmts, res) => {
                let items = self.prog.stmts[stmts.range()].to_vec();
                for st in items {
                    let TStmt::Val(_, init) = st else { return None };
                    if !self.pure_binding(init, projections) {
                        return None;
                    }
                }
                return self.fold_tree(res, projections);
            }
            TExpr::Local(s) | TExpr::Static(s) => return self.fold_value(s),
            TExpr::Field(r, s) => match self.prog.expr(r) {
                TExpr::New(c, args) if projections && (self.is_tuple_class(c) || self.syms.class(c).mods & mods::CASE != 0) => {
                    let i = self.syms.class(c).ctor_syms.first()?.iter().position(|&f| f == s)?;
                    let items = self.prog.expr_list(args).to_vec();
                    let folded: Vec<_> = items.iter().map(|&x| self.fold_tree(x, projections)).collect::<Option<_>>()?;
                    return folded.into_iter().nth(i);
                }
                _ => return self.fold_value(s),
            },
            TExpr::Unary(op, a) => match (op, self.fold_tree(a, projections)?) {
                (UnOp::IntNeg, Int(v)) => Int(v.wrapping_neg()),
                (UnOp::LongNeg, Long(v)) => Long(v.wrapping_neg()),
                (UnOp::DoubleNeg, Double(v)) => Double((-f64::from_bits(v)).to_bits()),
                (UnOp::DoubleNeg, Int(v)) => Double((-(v as f64)).to_bits()),
                (UnOp::BoolNot, Bool(v)) => Bool(!v),
                (UnOp::IntNot, Int(v)) => Int(!v),
                (UnOp::LongNot, Long(v)) => Long(!v),
                (UnOp::IntToLong, Int(v)) => Long(v as i64),
                // The same number: an `Int` constant stands where a `Double` is typed, as the
                // literal `1` typed `Double` does, and as the backends take it.
                (UnOp::IntToDouble | UnOp::ByteToShort | UnOp::ByteToInt | UnOp::ShortToInt, Int(v)) => Int(v),
                (UnOp::LongToDouble, Long(v)) => Double((v as f64).to_bits()),
                (UnOp::LongToInt, Long(v)) => Int(v as i32),
                (UnOp::DoubleToInt, Double(v)) => Int(f64::from_bits(v) as i32),
                (UnOp::DoubleToLong, Double(v)) => Long(f64::from_bits(v) as i64),
                (UnOp::CharToInt, Char(c)) => Int(c as i32),
                (UnOp::CharToLong, Char(c)) => Long(c as i64),
                (UnOp::IntToChar, Int(v)) => Char(v as u16),
                _ => return None,
            },
            TExpr::Prim(op, a, b) => {
                // `null == c` for a constant `c` is false, `null == null` true.
                let (a_null, b_null) = (matches!(self.prog.expr(a), TExpr::Null), matches!(self.prog.expr(b), TExpr::Null));
                if a_null || b_null {
                    let both = a_null && b_null || (a_null && self.fold_tree(b, projections).is_some()) || (b_null && self.fold_tree(a, projections).is_some());
                    if !both {
                        return None;
                    }
                    return match op {
                        PrimOp::Eq | PrimOp::RefEq => Some(Bool(a_null && b_null)),
                        PrimOp::Ne | PrimOp::RefNe => Some(Bool(!(a_null && b_null))),
                        _ => None,
                    };
                }
                let (x, y) = (self.fold_tree(a, projections)?, self.fold_tree(b, projections)?);
                if !same_tag_or_numeric(x, y) {
                    return None;
                }
                return fold_prim(op, x, y);
            }
            // Only strings concatenate to a constant; `"n=" + 1` stays a computation.
            TExpr::ToStr(a, _) => match self.fold_tree(a, projections)? {
                v @ Str(_) => v,
                _ => return None,
            },
            TExpr::StrConcat(l) if !self.interpolations.contains_key(&e) => {
                let items: Vec<TExprId> = self.prog.expr_list(l).to_vec();
                let mut out = String::new();
                for item in items {
                    match self.fold_tree(item, projections)? {
                        Str(s) => crate::text::push_str(&mut out, &self.name_str(s)),
                        _ => return None,
                    }
                }
                Str(self.interner.intern(&out))
            }
            _ => return None,
        })
    }

    /// A binding an inline match left: a constant, or a case class or tuple built from constants.
    fn pure_binding(&mut self, init: TExprId, projections: bool) -> bool {
        if self.fold_tree(init, projections).is_some() {
            return true;
        }
        match self.prog.expr(init) {
            TExpr::New(c, args) if self.is_tuple_class(c) || self.syms.class(c).mods & mods::CASE != 0 => {
                let items = self.prog.expr_list(args).to_vec();
                items.iter().all(|&x| self.fold_tree(x, projections).is_some())
            }
            _ => false,
        }
    }

    /// The constant a value stands for: one of a literal type, or a val of the library
    /// initialised with a literal (`Int.MaxValue`).
    pub(super) fn fold_value(&mut self, s: SymId) -> Option<LitVal> {
        let ty = self.syms.sym(s).sig.as_ref()?.ret;
        if let Some(v) = self.fold_type(ty) {
            return Some(v);
        }
        let info = self.syms.sym(s);
        // Not a product's: another module's val is a constant by its type alone, as the program's
        // own is in a whole build (`fold_type` above).
        if info.kind != SymKind::Val || info.owner == Owner::Local || !self.source(info.file).is_std || self.in_products(info.file) || self.inline.as_typed {
            return None;
        }
        self.ensure_body(s);
        let init = *self.val_init.get(&s)?;
        match self.prog.expr(init) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) => self.fold_constant(init),
            _ => None,
        }
    }

    /// A pure expression over the standard library, run by the interpreter under a small budget:
    /// `"ab".length + 1`, `List(1, 2).sum`. An effect (output, a mutation of shared state, a
    /// random number), an exception or an exhausted budget leaves the expression as it is. The
    /// interpreter is the one the macro expansions run in, with its caches (the tables over the
    /// IR, the module instances, the files initialised), which a fresh one had rebuilt per
    /// argument; a run that ran out of budget or depth may have left a module half built, so
    /// its caches are dropped.
    pub(super) fn fold_by_eval(&mut self, e: TExprId) -> Option<LitVal> {
        if self.inline.folding || !self.may_yield_literal(e) || !self.literal_type_possible(e) || no_fold() {
            return None;
        }
        let mut fractional = false;
        if !self.fold_candidate(e, &mut Vec::new(), &mut fractional) {
            return None;
        }
        self.inline.folding = true;
        let mark = self.diags.items.len();
        let t0 = self.profile.tick();
        let caches = crate::interp::take_caches();
        let mut interp = crate::interp::Interp::new(self, crate::interp::Limits { steps: FOLD_STEPS, depth: 64 });
        if let Some(c) = caches {
            interp.restore(*c);
        }
        interp.pure = true;
        let outer_epoch = crate::interp::enter_epoch();
        let run_guard = crate::interp::enter_run();
        let result = interp.eval_expr(e, None);
        drop(run_guard);
        crate::interp::leave_epoch(outer_epoch);
        let shared = crate::interp::take_touched();
        let steps = FOLD_STEPS - interp.steps_left();
        let mut keep_caches = true;
        let lit = match result {
            // A number formats differently on the two targets: a string made from one stays a
            // computation.
            Ok(v) => interp.as_literal(&v).filter(|l| !matches!(v, crate::interp::Value::Float(_) | crate::interp::Value::Byte(_) | crate::interp::Value::Short(_)) && !(fractional && matches!(l, LitVal::Str(_)))),
            Err(f) => {
                keep_caches = !matches!(f, crate::interp::Failure::Budget) && !interp.ended_on_depth(&f);
                if std::env::var_os("TEQ_INTERP_TRACE").is_some() {
                    eprintln!("fold: {}", interp.describe(&f));
                }
                None
            }
        };
        let caches = interp.into_caches();
        crate::interp::keep_caches(keep_caches.then(|| Box::new(caches)));
        if let Some(t) = shared {
            self.shared_state_changed(|| format!("a constant folded by evaluation changed {}, which the macros' runs share", t.what), t.module);
        }
        self.inline.folding = false;
        if let Some(t0) = t0 {
            self.profile.inline.fold_interp_ns += t0.elapsed().as_nanos() as u64;
            self.profile.inline.fold_runs += 1;
            self.profile.inline.fold_steps += steps;
            self.profile.inline.fold_literals += lit.is_some() as u64;
        }
        // What the interpreter asked the typer for was typed on its behalf.
        if self.diags.items.len() > mark {
            self.drop_reported_since(mark);
            return None;
        }
        lit
    }

    /// Whether the static type of `e` admits a literal: a literal type, a class a literal has
    /// or one above it (`Any`, `AnyVal`, `Comparable`), a type that says less (a type variable,
    /// a union). A `Map` or a `List` never folds, and its evaluation would be wasted.
    fn literal_type_possible(&mut self, e: TExprId) -> bool {
        let Some(ty) = self.prog.type_of(e) else { return true };
        let t = self.dealias(ty);
        let Type::Class(c, _) = self.types.get(t) else { return true };
        let b = &self.b;
        let literal_classes = [b.int, b.long, b.double, b.boolean, b.char, b.string];
        literal_classes.iter().any(|&l| l == c || self.syms.class(l).base_types.iter().any(|&(base, _)| base == c))
    }

    /// Whether the value of `e` can be a literal at all: an instance, a function or a sequence
    /// cannot, and an `inline match` folds its tuple scrutinee at every case.
    fn may_yield_literal(&self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::New(..) | TExpr::NewVia(..) | TExpr::Lambda(..) | TExpr::SeqLit(_) | TExpr::ArrayLit(_) | TExpr::Module(_) | TExpr::ClassOf(_) | TExpr::Unit | TExpr::Null => false,
            TExpr::Block(_, res) => self.may_yield_literal(res),
            _ => true,
        }
    }

    /// Whether `e` is literals and calls of the standard library: nothing of the program, whose
    /// definitions are typed in their own order, no local other than one bound inside `e`, and
    /// no application of a function value, which scalac does not reduce either. Nothing whose
    /// value differs between the targets either: a class or its name, a hash code, a member of
    /// the platform layers. `fractional` notes a Double or Float on the way, whose formatting
    /// differs too.
    fn fold_candidate(&mut self, e: TExprId, bound: &mut Vec<SymId>, fractional: &mut bool) -> bool {
        // Another module's products are the program's, which the whole build does not run.
        let std_sym = |t: &mut Self, s: SymId| t.source(t.syms.sym(s).file).is_std && !t.in_products(t.syms.sym(s).file) && t.same_on_both_targets(s);
        let std_class = |t: &Self, c: ClassId| t.source(t.syms.class(c).file).is_std && !t.in_products(t.syms.class(c).file);
        let list = |t: &mut Self, l: ListRef, bound: &mut Vec<SymId>, fractional: &mut bool| -> bool {
            let items = t.prog.expr_list(l).to_vec();
            items.into_iter().all(|a| t.fold_candidate(a, bound, fractional))
        };
        match self.prog.expr(e) {
            TExpr::Double(_) => {
                *fractional = true;
                true
            }
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null => true,
            TExpr::Local(s) => bound.contains(&s),
            TExpr::Static(s) => std_sym(self, s) && self.note_fractional(s, fractional),
            TExpr::Module(c) => std_class(self, c),
            TExpr::ClassOf(_) => false,
            TExpr::Field(r, s) => std_sym(self, s) && self.note_fractional(s, fractional) && self.fold_candidate(r, bound, fractional),
            TExpr::CallStatic(s, args) => std_sym(self, s) && self.note_fractional(s, fractional) && list(self, args, bound, fractional),
            TExpr::CallMethod(r, s, args) => {
                std_sym(self, s) && self.note_fractional(s, fractional) && self.fold_candidate(r, bound, fractional) && list(self, args, bound, fractional)
            }
            TExpr::New(c, args) => std_class(self, c) && list(self, args, bound, fractional),
            TExpr::NewVia(s, args) => std_sym(self, s) && list(self, args, bound, fractional),
            // The typer's own templates for a hash code and a class (`$hashCode($0)`,
            // `$identityHash($0)`, `$getClass($0)`) are the target's.
            TExpr::Js(template, args) => {
                let text = &self.prog.strings[template.idx()];
                if text.contains("Hash") || text.contains("hashCode") || text.contains("Class") {
                    return false;
                }
                let def = self.prog.template_syms.get(&template).copied();
                def.map_or(true, |s| std_sym(self, s) && self.note_fractional(s, fractional)) && list(self, args, bound, fractional)
            }
            TExpr::Lambda(params, body) => {
                let mark = bound.len();
                bound.extend_from_slice(self.prog.sym_list(params));
                let ok = self.fold_candidate(body, bound, fractional);
                bound.truncate(mark);
                ok
            }
            TExpr::Block(stmts, res) => {
                let mark = bound.len();
                let items = self.prog.stmts[stmts.range()].to_vec();
                let mut ok = true;
                for st in items {
                    ok = ok && match st {
                        TStmt::Expr(x) => self.fold_candidate(x, bound, fractional),
                        TStmt::Val(sym, init) => {
                            let fine = self.fold_candidate(init, bound, fractional);
                            bound.push(sym);
                            fine
                        }
                        TStmt::Fun(_) | TStmt::Pat(..) => false,
                    };
                }
                let ok = ok && self.fold_candidate(res, bound, fractional);
                bound.truncate(mark);
                ok
            }
            TExpr::If(c, t, els) => {
                self.fold_candidate(c, bound, fractional) && self.fold_candidate(t, bound, fractional) && els.map_or(true, |x| self.fold_candidate(x, bound, fractional))
            }
            TExpr::Prim(op, a, b) => {
                if matches!(op, PrimOp::DoubleAdd | PrimOp::DoubleSub | PrimOp::DoubleMul | PrimOp::DoubleDiv | PrimOp::DoubleRem | PrimOp::FloatAdd | PrimOp::FloatSub | PrimOp::FloatMul | PrimOp::FloatDiv | PrimOp::FloatRem) {
                    *fractional = true;
                }
                self.fold_candidate(a, bound, fractional) && self.fold_candidate(b, bound, fractional)
            }
            TExpr::Unary(op, a) => {
                if matches!(op, UnOp::IntToFloat | UnOp::IntToDouble | UnOp::LongToFloat | UnOp::DoubleToFloat | UnOp::LongToDouble | UnOp::FloatToDouble) {
                    *fractional = true;
                }
                self.fold_candidate(a, bound, fractional)
            }
            TExpr::ToStr(a, conv) => {
                if conv.kind() == StrKind::Double {
                    *fractional = true;
                }
                self.fold_candidate(a, bound, fractional)
            }
            TExpr::TypeTest(a, _) | TExpr::Cast(a, ..) | TExpr::Index(a, _) => self.fold_candidate(a, bound, fractional),
            TExpr::StrConcat(l) | TExpr::SeqLit(l) | TExpr::ArrayLit(l) => list(self, l, bound, fractional),
            _ => false,
        }
    }

    /// Whether a member's value is the same on JavaScript and the JVM: not a class, its name
    /// or a hash code, and not of the platform layers (`java.*`, `scala.scalajs.*`), whose
    /// members stand for one target.
    fn same_on_both_targets(&mut self, s: SymId) -> bool {
        let name = self.name_str(self.syms.sym(s).name);
        if matches!(name.as_str(), "getClass" | "hashCode" | "##" | "getName" | "getSimpleName" | "identityHashCode" | "toHexString" | "toOctalString" | "toBinaryString") {
            return false;
        }
        let mut owner = self.syms.sym(s).owner;
        let pkg = loop {
            match owner {
                Owner::Package(p) => break p,
                Owner::Class(c) => owner = self.syms.class(c).owner,
                Owner::Local => return true,
            }
        };
        !self.under_platform_package(pkg)
    }

    /// Whether a package is `java`, `js` or `scala.scalajs`, or lies under one of them.
    fn under_platform_package(&self, pkg: PkgId) -> bool {
        let mut at = Some(pkg);
        while let Some(p) = at.filter(|&p| p != ROOT_PKG) {
            if self.pkg_is(p, "java") || self.pkg_is(p, "js") || self.pkg_is(p, "scala.scalajs") {
                return true;
            }
            at = self.syms.pkg(p).parent;
        }
        false
    }

    /// Notes a member whose result is a Double or Float.
    fn note_fractional(&mut self, s: SymId, fractional: &mut bool) -> bool {
        if let Some(ret) = self.syms.sym(s).sig.as_ref().map(|sig| sig.ret) {
            let ret = self.dealias(ret);
            if ret == self.b.t_double || ret == self.b.t_float {
                *fractional = true;
            }
        }
        true
    }

    /// The constant of a literal type.
    /// scalac's `ConstantValue` of a typed tree: a constant as a tree, or a type that is one once
    /// a path's type is widened to what it stands for, dealiased and normalised (`one()` of `def
    /// one(): 1`, `cond()` of `def cond(): C.b.type` over a `final val b: true`).
    pub(super) fn constant_tree_value(&mut self, e: TExprId, ty: Option<TypeId>) -> Option<LitVal> {
        if let Some(v) = self.fold_constant(e) {
            return Some(v);
        }
        self.constant_by_type(e, ty)
    }

    /// The constant the type of the tree `e` stands for (`constant_tree_value`): its type `ty`
    /// where the copy gives one, or what a call or selection of it is declared to answer, which
    /// a substituted argument's binding widened (`one(0)` bound to an `inline x: Int`).
    pub(super) fn constant_by_type(&mut self, e: TExprId, ty: Option<TypeId>) -> Option<LitVal> {
        if let Some(v) = ty.or_else(|| self.prog.type_of(e)).and_then(|t| self.constant_type_value(t)) {
            return Some(v);
        }
        let (declared, args) = match self.prog.expr(e) {
            TExpr::CallStatic(s, args) | TExpr::CallMethod(_, s, args) => (s, args),
            TExpr::Static(s) | TExpr::Field(_, s) => (s, ListRef::EMPTY),
            _ => return None,
        };
        // The result type the call is declared with, its parameters' paths the arguments' (`one(0)`
        // of `def one(n: Int): 1`, `same(3)` of `def same(x: Int): x.type`).
        let sig = self.sig_arc(declared);
        let params: Vec<SymId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.sym)).collect();
        let args: Vec<TExprId> = self.prog.expr_list(args).to_vec();
        let mut ret = sig.ret;
        if params.iter().any(|&p| self.mentions_term(ret, p)) {
            let mut paths: Vec<(SymId, TypeId)> = Vec::with_capacity(params.len());
            for (&p, &a) in params.iter().zip(&args) {
                let path = match self.fold_constant(a) {
                    Some(v) => Some(self.types.lit(v)),
                    None => self.argument_path(a),
                };
                paths.extend(path.map(|path| (p, path)));
            }
            ret = self.subst_paths(ret, &paths);
        }
        self.constant_type_value(ret)
    }

    /// The constant a type stands for, a path's type widened to what it stands for.
    pub(super) fn constant_type_value(&mut self, ty: TypeId) -> Option<LitVal> {
        let widened = self.widen_path(ty);
        self.fold_type(widened)
    }

    pub fn fold_type(&mut self, ty: TypeId) -> Option<LitVal> {
        let ty = self.normalize(ty);
        let ty = self.deref(ty);
        match self.types.get(ty) {
            Type::Lit(l) => Some(self.types.lit_val(l)),
            Type::Member(prefix, name) => match self.type_member(prefix, name)? {
                super::members::MemberInfo::Alias(a) if a != ty => self.fold_type(a),
                _ => None,
            },
            // `"Slot" & String`, as a bound member seen through its refinement: the constant
            // member is the intersection when it conforms to the other.
            Type::Inter(a, b) => {
                for (x, y) in [(a, b), (b, a)] {
                    if let Some(v) = self.fold_type(x) {
                        if self.is_sub(x, y) {
                            return Some(v);
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// A constant as an expression with its literal type.
    pub(super) fn constant(&mut self, v: LitVal) -> (TExprId, TypeId) {
        let te = self.literal_expr(v);
        let ty = self.types.lit(v);
        self.prog.set_type(te, ty);
        (te, ty)
    }

    // ---- inline if and match ----

    pub(super) fn type_inline_if(&mut self, c: ExprId, t: ExprId, els: Option<ExprId>, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        if self.checks_inline_definition() {
            let r = self.type_plain_if(c, t, els, expected);
            let cond = self.cur_ast().expr_span(c);
            self.note_reducible(r.0, ReducibleSource::If { whole: span, cond });
            return r;
        }
        // A retained body is the method inlined into itself with its own parameters (scalac's
        // `Inliner` making the dispatch method), so its `inline if` reduces as an expansion's does.
        let retained = self.inline.depth == 0 && self.inline.retained > 0;
        if self.inline.depth == 0 && !retained {
            self.error(span, "inline if can only be used in an inline method");
        }
        let (raw, cond_ty) = self.type_expr(c, Some(self.b.t_boolean));
        let tc = self.adapt(raw, cond_ty, self.b.t_boolean, self.cur_ast().expr_span(c));
        // scalac reduces on a condition of a literal type too (`valueOf[debug]`), keeping the
        // condition before the branch it selects.
        let reads = self.inline.leaf_reads;
        let (folded, kept) = match self.fold_constant(tc) {
            Some(v) => (Some(v), None),
            None => (self.fold_type(cond_ty), Some(tc)),
        };
        let read_leaf = self.inline.leaf_reads != reads;
        // Under an erased scrutinee's binders the branch dropped is checked as the taken one is
        // (`cleanupUnusable` sees the case's body before its `inline if` reduces).
        if !self.inline.unusable.is_empty() {
            match folded {
                Some(LitVal::Bool(true)) => els.into_iter().for_each(|e| self.check_dropped_unusable(e, span)),
                Some(LitVal::Bool(false)) => self.check_dropped_unusable(t, span),
                _ => {}
            }
        }
        let (te, ty) = match folded {
            Some(LitVal::Bool(true)) => self.type_expr_adapted(t, expected),
            Some(LitVal::Bool(false)) => match els {
                Some(e) => self.type_expr_adapted(e, expected),
                None => (self.prog.add(TExpr::Unit), self.b.t_unit),
            },
            _ => {
                if self.inline.depth > 0 || retained {
                    let text = self.source_text(self.env.file, self.cur_ast().expr_span(c));
                    let mut msg = format!("Cannot reduce `inline if` because its condition is not a constant value: {}", text);
                    if retained {
                        let f = self.source(self.env.file);
                        let (line, _, _) = crate::source::locate(&f.text, span.start as usize);
                        msg.push_str(&format!("\n  inlined from {}:{}", f.path, line));
                        // scalac makes the dispatch method in its `Inlining`, after the typer.
                        let file = self.env.file;
                        self.diags.late_error(file, span, msg);
                    } else {
                        self.error(span, msg);
                    }
                }
                return (self.prog.add(TExpr::Unit), ERROR);
            }
        };
        if folded.is_some() {
            self.mark_folded_leaf(te, |t| read_leaf || (kept.is_some() && t.leaf_near(tc)));
        }
        match kept {
            Some(cond) => {
                let stmts = self.prog.stmts.push_slice(&[TStmt::Expr(cond)]);
                let block = self.prog.add(TExpr::Block(stmts, te));
                self.prog.set_type(block, ty);
                (block, ty)
            }
            None => (te, ty),
        }
    }

    pub(super) fn type_inline_match(&mut self, scrut: ExprId, cases: ListRef, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        if self.checks_inline_definition() {
            // Every case is typed against the scrutinee's type at the definition; which of them
            // the static type rules out is settled where the method expands.
            self.inline.lenient_match += 1;
            let r = self.type_match(scrut, cases, span, expected);
            self.inline.lenient_match -= 1;
            let ast = self.cur_ast();
            let sources = ast.case_list(cases).iter().map(|c| (ast.pat_spans[c.pat.idx()], c.guard.map(|g| ast.expr_span(g)))).collect();
            self.note_reducible(r.0, ReducibleSource::Match { whole: span, scrut: ast.expr_span(scrut), cases: sources });
            return r;
        }
        // A retained body is the method inlined into itself with its own parameters, so its
        // `inline match` reduces on their declared types, as an expansion's does on the
        // arguments' (scalac: `pick(x: Any)`'s `case _: Int` never matches there).
        let retained = self.inline.depth == 0 && self.inline.retained > 0;
        if self.inline.depth == 0 && !retained {
            self.error(span, "inline match can only be used in an inline method");
            return self.type_match(scrut, cases, span, expected);
        }
        let ast = self.cur_ast();
        let (ts, sty) = self.type_expr(scrut, None);
        let sty = self.solve_in(sty);
        let constant = self.fold_constant(ts).or_else(|| self.fold_type(sty));
        // A scrutinee with an `erasedValue` anywhere in it is erased (`existsSubTree(_.symbol.isErased)`).
        let mut parts = self.erased_tested(ts);
        let marked = self.inline.erased.take() == Some(ts);
        if marked {
            parts.push(ts);
        }
        let erased = marked || !parts.is_empty() || self.names_erased_value(ts);
        let erased_parts: std::rc::Rc<[TExprId]> = parts.into();
        let mut scrutinee = Scrutinee { expr: ts, ty: sty, constant, erased, bound: None, erased_parts };
        let clauses = ast.case_list(cases).to_vec();
        let unusable_mark = self.inline.unusable.len();
        for c in &clauses {
            self.push_scope();
            let tparam_mark = self.inline.tparams.len();
            self.inline.unusable.truncate(unusable_mark);
            let mut stmts: Vec<TStmt> = Vec::new();
            scrutinee.bound = None;
            // Each case an attempt: one that does not match goes whole, its constraints with it.
            let attempt = self.attempt();
            let part = self.part(Part::Match);
            // A matching case whose guard is a constant `false` gives way to the next; one whose
            // guard is no constant stops the reduction (scalac's `InlineReducer.reduceCase`).
            let mut stuck = false;
            let matched = self.inline_pattern(c.pat, &mut scrutinee, &mut stmts)
                && match c.guard {
                    Some(g) => {
                        let tg = self.check_expr(g, self.b.t_boolean);
                        match self.constant_tree_value(tg, None) {
                            Some(LitVal::Bool(b)) => b,
                            _ => {
                                stuck = true;
                                false
                            }
                        }
                    }
                    None => true,
                };
            self.part_end(part);
            if !matched {
                self.retract(attempt);
                self.truncate_tparams(tparam_mark);
                self.pop_scope();
                if stuck {
                    break;
                }
                continue;
            }
            self.close(attempt);
            let (body, ty) = self.type_expr_adapted(c.body, expected);
            self.truncate_tparams(tparam_mark);
            self.pop_scope();
            // The other cases, which the reduction drops, under an enclosing erased scrutinee's
            // binders: checked as the selected one is.
            if unusable_mark > 0 {
                for other in clauses.iter().filter(|o| o.body != c.body) {
                    self.check_dropped_unusable(other.body, span);
                }
            }
            self.report_unusable_reads(unusable_mark, body, &[], span);
            self.drop_unusable(unusable_mark, &mut stmts);
            // The scrutinee is evaluated once where no binder of the case reads it and it is not
            // elideable, as scalac's `InlineReducer` binds it (`inline { println(..); x } match`,
            // an inline or by-name parameter).
            if scrutinee.bound.is_none() && !scrutinee.erased && !self.elideable_scrutinee(scrutinee.expr) {
                stmts.insert(0, TStmt::Expr(scrutinee.expr));
            }
            if stmts.is_empty() {
                return (body, ty);
            }
            let l = self.prog.stmts.push_slice(&stmts);
            return (self.prog.add(TExpr::Block(l, body)), ty);
        }
        let mut msg = format!(
            "cannot reduce inline match with\n scrutinee:  {} : {}\n patterns :",
            self.source_text(self.env.file, ast.expr_span(scrut)),
            self.show(sty)
        );
        for (i, c) in clauses.iter().enumerate() {
            let text = self.source_text(self.env.file, ast.pat_spans[c.pat.idx()]);
            msg.push_str(if i == 0 { "  case " } else { "\n             case " });
            msg.push_str(&text);
            if let Some(g) = c.guard {
                msg.push_str(" if ");
                msg.push_str(&self.source_text(self.env.file, ast.expr_span(g)));
            }
        }
        if retained {
            let f = self.source(self.env.file);
            let (line, _, _) = crate::source::locate(&f.text, span.start as usize);
            msg.push_str(&format!("\n  inlined from {}:{}", f.path, line));
            let file = self.env.file;
            self.diags.late_error(file, span, msg);
        } else {
            self.error(span, msg);
        }
        (self.prog.add(TExpr::Unit), ERROR)
    }

    /// The scrutinee's value as a stable reference, bound to a local of `stmts` on first use.
    fn scrutinee_ref(&mut self, s: &mut Scrutinee, stmts: &mut Vec<TStmt>, span: Span) -> TExprId {
        if let Some(b) = s.bound {
            return self.copy_expr(b);
        }
        if s.erased {
            return self.prog.add(TExpr::Unit);
        }
        if self.is_stable(s.expr) {
            return self.copy_expr(s.expr);
        }
        let local = self.indexed_local("scrutinee", stmts.len() as u32, s.ty, span);
        stmts.push(TStmt::Val(local, s.expr));
        let r = self.prog.add(TExpr::Local(local));
        s.bound = Some(r);
        r
    }

    /// Whether the pattern matches the scrutinee statically, binding what it names into the
    /// current scope and `stmts`. A pattern that does not match leaves no diagnostics.
    fn inline_pattern(&mut self, p: PatId, s: &mut Scrutinee, stmts: &mut Vec<TStmt>) -> bool {
        let mark = self.diags.items.len();
        let matched = self.inline_pattern_now(p, s, stmts);
        if !matched {
            self.drop_reported_since(mark);
        }
        matched
    }

    fn inline_pattern_now(&mut self, p: PatId, s: &mut Scrutinee, stmts: &mut Vec<TStmt>) -> bool {
        let ast = self.cur_ast();
        let span = ast.pat_spans[p.idx()];
        match ast.pat(p) {
            Pat::Wildcard | Pat::Error => true,
            Pat::Bind(name, inner) => {
                let ty = match inner {
                    Some(i) => {
                        if !self.inline_pattern(i, s, stmts) {
                            return false;
                        }
                        self.inline_pattern_type(i, s.ty)
                    }
                    None => s.ty,
                };
                let sym = self.new_local(name, SymKind::Val, ty, span);
                // A binder over a constant stands for the constant, as scalac's reduced
                // projection does, so that a guard on it folds; one over an erased scrutinee is
                // erased and unusable (`adjustErased`), whatever its type.
                if s.erased {
                    self.inline.unusable.push(sym);
                }
                match s.constant.filter(|_| !s.erased) {
                    Some(v) => {
                        let (lit, _) = self.constant(v);
                        self.inline.args.insert(sym, InlineArg { expr: lit, ty, source: None });
                    }
                    None => {
                        let value = self.scrutinee_ref(s, stmts, span);
                        stmts.push(TStmt::Val(sym, value));
                    }
                }
                self.bind_local(name, sym);
                true
            }
            Pat::Typed(inner, ty) => {
                let Some(narrowed) = self.type_pattern_matches(ty, s.ty) else { return false };
                let narrowed = if matches!(ast.pat(inner), Pat::Wildcard) { narrowed } else { self.narrowed_to(s.ty, narrowed) };
                let mut narrowed_scrutinee = Scrutinee { expr: s.expr, ty: narrowed, constant: s.constant, erased: s.erased, bound: s.bound, erased_parts: s.erased_parts.clone() };
                let ok = self.inline_pattern(inner, &mut narrowed_scrutinee, stmts);
                s.bound = narrowed_scrutinee.bound;
                ok
            }
            Pat::Lit(e) => {
                let (te, _) = self.type_expr(e, Some(s.ty));
                if matches!(self.prog.expr(te), TExpr::Null) {
                    return matches!(self.prog.expr(s.expr), TExpr::Null);
                }
                match (self.fold_constant(te), s.constant) {
                    (Some(a), Some(b)) => a == b || same_number(a, b),
                    _ => false,
                }
            }
            Pat::StableId(path) => {
                let (te, pty) = self.type_expr(path, None);
                match (self.prog.expr(te), self.prog.expr(s.expr)) {
                    (TExpr::Module(a), TExpr::Module(b)) => a == b,
                    (TExpr::Module(_), _) => {
                        let mark = self.snapshot();
                        let ok = self.is_sub(s.ty, pty);
                        self.rollback(mark);
                        ok
                    }
                    _ => match (self.fold_constant(te), s.constant) {
                        (Some(a), Some(b)) => a == b,
                        _ => false,
                    },
                }
            }
            Pat::Tuple(subs) => {
                let subs = ast.pat_list(subs).to_vec();
                let sty = self.dealias(s.ty);
                let Type::Class(c, args) = self.types.get(sty) else { return false };
                if !self.is_tuple_class(c) || self.types.items(args).len() != subs.len() {
                    return false;
                }
                self.inline_field_patterns(c, args, &subs, s, stmts, span)
            }
            Pat::Ctor(path, subs) => {
                let subs = ast.pat_list(subs).to_vec();
                if let Expr::Ident(n) = ast.expr(path) {
                    if self.name_ref(n) == "*:" && subs.len() == 2 {
                        return self.inline_cons_pattern(subs[0], subs[1], s, stmts, span);
                    }
                }
                let Some(c) = self.inline_pattern_class(path) else { return false };
                let (class_ty, _) = self.instantiate_pattern_class(c, s.ty);
                let mark = self.snapshot();
                let conforms = self.is_sub(s.ty, class_ty);
                if !conforms {
                    self.rollback(mark);
                    return false;
                }
                let class_ty = self.zonk(class_ty);
                let Type::Class(_, args) = self.types.get(class_ty) else { return false };
                if self.syms.class(c).ctor_syms.first().map_or(0, |f| f.len()) != subs.len() {
                    return false;
                }
                self.inline_field_patterns(c, args, &subs, s, stmts, span)
            }
            Pat::Alt(alts) => {
                let alts = ast.pat_list(alts).to_vec();
                alts.into_iter().any(|a| self.inline_pattern(a, s, stmts))
            }
            Pat::Rest(_) | Pat::Quote(_) | Pat::QuoteType(_) | Pat::NamedField(..) | Pat::Interp(..) => false,
        }
    }

    fn inline_pattern_class(&mut self, path: ExprId) -> Option<ClassId> {
        let c = self.pattern_class(path, false)?;
        self.complete_class(c);
        (self.syms.class(c).mods & mods::CASE != 0 && self.syms.class(c).kind != ClassKind::Object).then_some(c)
    }

    fn inline_field_patterns(&mut self, c: ClassId, args: TList, subs: &[PatId], s: &mut Scrutinee, stmts: &mut Vec<TStmt>, span: Span) -> bool {
        let fields: Vec<SymId> = self.syms.class(c).ctor_syms.concat();
        let subst: Subst = self.syms.class(c).tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
        for (i, &sub) in subs.iter().enumerate() {
            let field = fields[i];
            let fty = self.sig_of(field).ret;
            let fty = self.types.subst(fty, &subst);
            let constant = match self.prog.expr(s.expr) {
                TExpr::New(k, items) if k == c => {
                    let item = self.prog.expr_list(items)[i];
                    self.fold_constant(item)
                }
                _ => self.fold_type(fty),
            };
            if matches!(self.cur_ast().pat(sub), Pat::Wildcard) {
                continue;
            }
            let (expr, erased) = match self.erased_free_projection(s.expr, s.erased, &s.erased_parts, c, i) {
                Some(item) => (self.copy_expr(item), false),
                None => {
                    let value = self.scrutinee_ref(s, stmts, span);
                    let expr = if self.is_tuple_class(c) && fields.len() > 22 {
                        let elems: Vec<TypeId> = self.types.items(args).to_vec();
                        self.tuple_element(value, &fields, &elems, i, span)
                    } else {
                        self.prog.add(TExpr::Field(value, field))
                    };
                    (expr, s.erased)
                }
            };
            let mut elem = Scrutinee { expr, ty: fty, constant, erased, bound: None, erased_parts: s.erased_parts.clone() };
            if !self.inline_pattern(sub, &mut elem, stmts) {
                return false;
            }
        }
        true
    }

    /// `case h *: t` on a tuple value: the head and the tuple of the other elements.
    fn inline_cons_pattern(&mut self, head: PatId, tail: PatId, s: &mut Scrutinee, stmts: &mut Vec<TStmt>, span: Span) -> bool {
        let sty = self.dealias(s.ty);
        let Type::Class(c, args) = self.types.get(sty) else { return false };
        if !self.is_tuple_class(c) {
            return false;
        }
        let elems: Vec<TypeId> = self.types.items(args).to_vec();
        let fields: Vec<SymId> = self.syms.class(c).ctor_syms.concat();
        let value = self.scrutinee_ref(s, stmts, span);
        let head_expr = self.tuple_element(value, &fields, &elems, 0, span);
        let mut head_scrutinee = Scrutinee { expr: head_expr, ty: elems[0], constant: self.fold_type(elems[0]), erased: s.erased, bound: None, erased_parts: s.erased_parts.clone() };
        if !self.inline_pattern(head, &mut head_scrutinee, stmts) {
            return false;
        }
        let rest: Vec<TExprId> = (1..elems.len())
            .map(|i| {
                let v = self.scrutinee_ref(s, stmts, span);
                self.tuple_element(v, &fields, &elems, i, span)
            })
            .collect();
        let tail_ty = self.tuple_of(&elems[1..]);
        let tail_expr = self.tuple_value(&rest);
        let mut tail_scrutinee = Scrutinee { expr: tail_expr, ty: tail_ty, constant: None, erased: s.erased, bound: None, erased_parts: s.erased_parts.clone() };
        self.inline_pattern(tail, &mut tail_scrutinee, stmts)
    }

    /// The type a pattern narrows the scrutinee to: the type of a typed pattern, the class of
    /// a constructor pattern, the scrutinee's type otherwise.
    fn inline_pattern_type(&mut self, p: PatId, sty: TypeId) -> TypeId {
        let ast = self.cur_ast();
        match ast.pat(p) {
            Pat::Typed(_, ty) => match self.type_pattern_matches(ty, sty) {
                Some(pt) => self.narrowed_to(sty, pt),
                None => sty,
            },
            Pat::Ctor(path, _) => match self.inline_pattern_class(path) {
                Some(c) => {
                    let (t, _) = self.instantiate_pattern_class(c, sty);
                    self.zonk(t)
                }
                None => sty,
            },
            _ => sty,
        }
    }

    /// What a typed pattern narrows the scrutinee to: the scrutinee's own type where it says
    /// more than the pattern, as scalac's binder has it (a mirror's members are in the given's
    /// type, not in `Mirror.Of[T]`), the pattern type otherwise.
    pub(super) fn narrowed_to(&mut self, sty: TypeId, pt: TypeId) -> TypeId {
        let mark = self.snapshot();
        let precise = self.is_sub(sty, pt);
        self.rollback(mark);
        if precise { sty } else { pt }
    }

    /// Whether the scrutinee type conforms to the pattern type, binding the type variables of
    /// the pattern (`case _: (h *: t)`, `case _: List[t]`) as type names of the current scope.
    /// Returns the pattern type with its variables solved.
    fn type_pattern_matches(&mut self, ty: TyExprId, sty: TypeId) -> Option<TypeId> {
        let ast = self.cur_ast();
        // `A & B` matches a structural pattern that one of its parts matches, as `A & B <: P`
        // holds when `A <: P` or `B <: P` (`et & Tuple` of circe's derivation).
        let structural = matches!(ast.ty(ty), TyExpr::Tuple(_) | TyExpr::Apply(..));
        if structural {
            let head = self.dealias(sty);
            if let Type::Inter(a, b) = self.types.get(head) {
                return self.type_pattern_matches(ty, a).or_else(|| self.type_pattern_matches(ty, b));
            }
        }
        match ast.ty(ty) {
            TyExpr::TypeVar(n) => {
                self.bind_pattern_type_var(n, sty);
                Some(sty)
            }
            // `(P1, P2)` against a tuple of the same arity: each element pattern binds its own
            // variables to the element's exact type, a literal label included.
            TyExpr::Tuple(items) if items.len >= 2 => {
                let pats: Vec<TyExprId> = ast.ty_list(items).to_vec();
                self.tuple_pattern_matches(&pats, sty)
            }
            TyExpr::Apply(f, args) if self.pattern_head_is_cons(f) && args.len == 2 => {
                let [h, t] = *ast.ty_list(args) else { return None };
                let scrutinee = self.dealias(sty);
                let Type::Class(c, targs) = self.types.get(scrutinee) else { return None };
                if !self.is_tuple_class(c) {
                    return None;
                }
                let elems: Vec<TypeId> = self.types.items(targs).to_vec();
                let tail_ty = self.tuple_of(&elems[1..]);
                let head = self.type_pattern_matches(h, elems[0])?;
                let tail = self.type_pattern_matches(t, tail_ty)?;
                let mut all = vec![head];
                let tail = self.dealias(tail);
                if let Type::Class(tc, targs) = self.types.get(tail) {
                    if self.is_tuple_class(tc) {
                        all.extend_from_slice(&self.types.items(targs).to_vec());
                    }
                }
                Some(self.tuple_type(&all))
            }
            TyExpr::Apply(f, args) if args.len >= 2 && self.pattern_head_is_tuple(f, args.len as usize) => {
                let pats: Vec<TyExprId> = ast.ty_list(args).to_vec();
                self.tuple_pattern_matches(&pats, sty)
            }
            _ => {
                let mark = self.inline.pat_vars.len();
                let mut vars = Vec::new();
                self.collect_type_vars(ty, &mut vars);
                for n in &vars {
                    let v = self.fresh_var();
                    self.inline.pat_vars.push((*n, v));
                }
                let pt = self.resolve_type(ty);
                let snapshot = self.snapshot();
                let ok = pt != ERROR && self.is_sub(sty, pt);
                let result = if ok {
                    for i in mark..self.inline.pat_vars.len() {
                        let (n, v) = self.inline.pat_vars[i];
                        let solved = self.solve_if_var(v);
                        self.bind_pattern_type_var(n, solved);
                    }
                    Some(self.zonk(pt))
                } else {
                    self.rollback(snapshot);
                    None
                };
                self.inline.pat_vars.truncate(mark);
                result
            }
        }
    }

    fn tuple_pattern_matches(&mut self, pats: &[TyExprId], sty: TypeId) -> Option<TypeId> {
        let scrutinee = self.dealias(sty);
        let Type::Class(c, targs) = self.types.get(scrutinee) else { return None };
        if !self.is_tuple_class(c) || self.types.items(targs).len() != pats.len() {
            return None;
        }
        let elems: Vec<TypeId> = self.types.items(targs).to_vec();
        let mut matched = Vec::with_capacity(elems.len());
        for (&p, e) in pats.iter().zip(elems) {
            matched.push(self.type_pattern_matches(p, e)?);
        }
        Some(self.tuple_type(&matched))
    }

    /// The head of a pattern type is `*:`, written by name or as the resolved constructor a
    /// converted library body carries.
    fn pattern_head_is_cons(&mut self, f: TyExprId) -> bool {
        match self.cur_ast().ty(f) {
            TyExpr::Name(op) => self.name_ref(op) == "*:",
            TyExpr::Resolved(t) => matches!(self.types.get(t), Type::Ctor(c) if Some(c) == self.b.cons_tuple),
            _ => false,
        }
    }

    fn pattern_head_is_tuple(&mut self, f: TyExprId, arity: usize) -> bool {
        match self.cur_ast().ty(f) {
            TyExpr::Resolved(t) => matches!(self.types.get(t), Type::Ctor(c) if self.is_tuple_class(c) && self.syms.class(c).tparams.len() == arity),
            _ => false,
        }
    }

    pub(super) fn collect_type_vars(&self, ty: TyExprId, out: &mut Vec<Name>) {
        let ast = self.cur_ast();
        match ast.ty(ty) {
            TyExpr::TypeVar(n) => {
                if !out.contains(&n) {
                    out.push(n);
                }
            }
            TyExpr::Apply(f, args) => {
                self.collect_type_vars(f, out);
                for &a in ast.ty_list(args) {
                    self.collect_type_vars(a, out);
                }
            }
            TyExpr::Tuple(items) | TyExpr::Fun(items, _) => {
                for &a in ast.ty_list(items) {
                    self.collect_type_vars(a, out);
                }
                if let TyExpr::Fun(_, r) = ast.ty(ty) {
                    self.collect_type_vars(r, out);
                }
            }
            TyExpr::Union(a, b) | TyExpr::Inter(a, b) => {
                self.collect_type_vars(a, out);
                self.collect_type_vars(b, out);
            }
            _ => {}
        }
    }

    /// A type variable of a pattern becomes a type name of the case's scope, standing for the
    /// type it matched.
    fn bind_pattern_type_var(&mut self, n: Name, t: TypeId) {
        let p = self.syms.new_tparam(n, 0);
        if let Some(Frame::Locals { tparams, .. }) = self.env.frames.last_mut() {
            tparams.push((n, p));
        }
        self.bind_tparam(p, t);
    }

    /// The tuple type of these element types, `EmptyTuple` for none.
    pub fn tuple_of(&mut self, elems: &[TypeId]) -> TypeId {
        if elems.is_empty() {
            return self.empty_tuple_type();
        }
        self.tuple_type(elems)
    }

    pub fn empty_tuple_type(&mut self) -> TypeId {
        match self.empty_tuple_class() {
            Some(c) => self.types.class(c, &[]),
            None => self.b.t_unit,
        }
    }

    pub(super) fn empty_tuple_value(&mut self) -> TExprId {
        match self.empty_tuple_class() {
            Some(c) => self.prog.add(TExpr::Module(c)),
            None => self.prog.add(TExpr::Unit),
        }
    }

    pub(super) fn empty_tuple_class(&mut self) -> Option<ClassId> {
        let name = self.interner.intern("EmptyTuple");
        let scala = self.b.scala_pkg;
        match self.pkg_term(scala, name)? {
            super::resolve::TermRef::Global(s) | super::resolve::TermRef::ModuleMember(_, s) => match self.syms.sym(s).kind {
                SymKind::Object(c) => Some(c),
                _ => None,
            },
            _ => None,
        }
    }

    // ---- scala.compiletime ----

    /// The intrinsics that need the argument as written: `summonFrom` with its cases,
    /// `codeOf` and `requireConst` with the text of their argument.
    pub(super) fn early_intrinsic(&mut self, sym: SymId, lists: &[ArgList], span: Span, expected: Option<TypeId>) -> Option<(TExprId, TypeId)> {
        let intrinsic = self.intrinsic_of(sym)?;
        let arg = match lists.first().and_then(|l| l.args.first()) {
            Some(&ArgSrc::Ast(e)) => e,
            _ => return None,
        };
        // Under the definition check `codeOf` and `requireConst` stay calls of their argument,
        // which an expansion reads.
        if self.checks_inline_definition() {
            if intrinsic != Intrinsic::SummonFrom {
                return None;
            }
            if self.quote.level > 0 {
                self.note_held(HeldForm::SummonFrom);
                return Some((self.prog.add(TExpr::Unit), expected.unwrap_or(ERROR)));
            }
            return Some(self.summon_from_at_definition(arg, span, expected));
        }
        match intrinsic {
            Intrinsic::SummonFrom => {
                let part = self.part(Part::Intrinsic);
                let r = self.summon_from(arg, span, expected);
                self.part_end(part);
                Some(r)
            }
            Intrinsic::CodeOf => {
                let (file, at) = self.argument_source(arg);
                let text = self.source_text(file, at);
                let r = self.prog.add_str(&text);
                Some((self.prog.add(TExpr::Str(r)), self.b.t_string))
            }
            Intrinsic::RequireConst => {
                let (te, _) = self.type_expr(arg, None);
                // A plain call it holds pending is expanded for the check.
                if self.attempts.pending_len() != 0 {
                    self.expand_pending_in(&[te]);
                }
                if self.fold_constant(te).is_none() {
                    let (file, at) = self.argument_source(arg);
                    let text = self.source_text(file, at);
                    self.error(span, format!("expected a constant value but found: {}", text));
                }
                Some((self.prog.add(TExpr::Unit), self.b.t_unit))
            }
            _ => None,
        }
    }

    /// Where an argument was written: the argument of the enclosing inline call when it names
    /// a substituted parameter, the expression itself otherwise.
    fn argument_source(&mut self, e: ExprId) -> (FileId, Span) {
        let ast = self.cur_ast();
        if let Expr::Ident(n) = ast.expr(e) {
            if let Some(super::resolve::TermRef::Local(s)) = self.lookup_term(n) {
                if let Some(arg) = self.inline.args.get(&s) {
                    if let Some(source) = arg.source {
                        return source;
                    }
                }
            }
        }
        (self.env.file, ast.expr_span(e))
    }

    /// The cases of `summonFrom`'s argument, a pattern-matching anonymous function: none for
    /// another argument.
    fn summon_from_cases(&self, arg: ExprId) -> ListRef {
        let ast = self.cur_ast();
        let mut e = arg;
        loop {
            match ast.expr(e) {
                Expr::Parens(inner) => e = inner,
                Expr::Block(stmts) if stmts.len == 1 => match ast.stmt_list(stmts)[0] {
                    crate::ast::Stmt::Expr(inner) => e = inner,
                    _ => break,
                },
                _ => break,
            }
        }
        match ast.expr(e) {
            Expr::Lambda(_, body) => match ast.expr(body) {
                Expr::Match(_, cases) => cases,
                _ => ListRef::EMPTY,
            },
            _ => ListRef::EMPTY,
        }
    }

    /// The binder and the type of a case of `summonFrom`: `x: T`, `_: T`, `_`, `x`, or `given
    /// x: T` as a library body pickles it (the binder over the typed wildcard); `None` for
    /// another pattern.
    fn summon_from_case(&self, pat: PatId) -> Option<(Option<Name>, Option<TyExprId>)> {
        let ast = self.cur_ast();
        Some(match ast.pat(pat) {
            Pat::Typed(inner, ty) => match ast.pat(inner) {
                Pat::Bind(name, None) => (Some(name), Some(ty)),
                Pat::Wildcard => (None, Some(ty)),
                _ => return None,
            },
            Pat::Wildcard => (None, None),
            Pat::Bind(name, None) => (Some(name), None),
            Pat::Bind(name, Some(inner)) => match ast.pat(inner) {
                Pat::Typed(w, ty) if matches!(ast.pat(w), Pat::Wildcard) => (Some(name), Some(ty)),
                Pat::Wildcard => (Some(name), None),
                _ => return None,
            },
            _ => return None,
        })
    }

    /// A case of `summonFrom` as a program writes it, scalac's rule: `x: T`, `_: T`, `given T`
    /// (the parser's binder over the typed wildcard) or `_`.
    fn summon_from_written_case(&self, pat: PatId) -> Option<(Option<Name>, Option<TyExprId>)> {
        let ast = self.cur_ast();
        match ast.pat(pat) {
            Pat::Typed(inner, ty) => match ast.pat(inner) {
                Pat::Bind(name, None) => Some((Some(name), Some(ty))),
                Pat::Wildcard => Some((None, Some(ty))),
                _ => None,
            },
            Pat::Bind(name, Some(inner)) if ast.given_binds.contains(&pat) => match ast.pat(inner) {
                Pat::Typed(w, ty) if matches!(ast.pat(w), Pat::Wildcard) => Some((Some(name), Some(ty))),
                _ => None,
            },
            Pat::Wildcard => Some((None, None)),
            _ => None,
        }
    }

    /// Under the definition check, `summonFrom`'s cases typed where the body is defined, as
    /// scalac types the anonymous function it takes: each case's pattern type resolved there,
    /// its binder a given of that type in scope for its guard and body (scalac reads `x: T` as
    /// `given x @ _: T`; the local is a plain val the scope holds as a given, as the retype
    /// path's is), each guard a `Boolean`, each body against the expected type. What is kept is
    /// a match the expansion reduces (`Worker::reduce_summon_from`), of a placeholder scrutinee
    /// over the cases, noted among the reducible nodes with the patterns and guards as written,
    /// for the message of an expansion that none satisfies. Its type: the expected type where
    /// one is given, the cases' join otherwise. A case of another pattern is scalac's E153 at
    /// the pattern, every such case reported.
    fn summon_from_at_definition(&mut self, arg: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let cases = self.summon_from_cases(arg);
        if cases.is_empty() {
            self.error(span, "summonFrom takes a pattern-matching anonymous function");
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        let ast = self.cur_ast();
        let clauses = ast.case_list(cases).to_vec();
        let mut shapes = Vec::with_capacity(clauses.len());
        for c in &clauses {
            match self.summon_from_written_case(c.pat) {
                Some(shape) => shapes.push(shape),
                None => self.error(ast.pat_spans[c.pat.idx()], "Unexpected pattern for summonFrom. Expected `x: T` or `_`"),
            }
        }
        if shapes.len() < clauses.len() {
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        // A guard over a type variable of its case's pattern (`case t: Fail[X, y] if
        // constValue[y] < ..`) names what the search solves the variable to, which the check
        // does not bind: the body is held back.
        let guards_type_vars = clauses.iter().zip(&shapes).any(|(c, &(_, ty))| {
            let mut vars = Vec::new();
            if let Some(ty) = ty.filter(|_| c.guard.is_some()) {
                self.collect_type_vars(ty, &mut vars);
            }
            !vars.is_empty()
        });
        if guards_type_vars {
            self.note_held(HeldForm::SummonFrom);
            return (self.prog.add(TExpr::Unit), expected.unwrap_or(ERROR));
        }
        let expected = self.concrete_expected(expected);
        let mut typed: Vec<TCase> = Vec::with_capacity(clauses.len());
        let mut joined: Option<TypeId> = None;
        let mut written = Vec::with_capacity(clauses.len());
        for (c, (binder, ty)) in clauses.iter().zip(shapes) {
            let pat_span = ast.pat_spans[c.pat.idx()];
            written.push((pat_span, c.guard.map(|g| ast.expr_span(g))));
            let pat = match ty {
                Some(ty) => {
                    let t = self.resolve_type(ty);
                    let always = self.prog.add_test(TypeTest::Always);
                    let wildcard = self.prog.add_pat(TPat::Wildcard);
                    let test = self.prog.add_pat(TPat::Test(always, t, wildcard));
                    match binder {
                        Some(name) => {
                            let sym = self.new_local(name, SymKind::Val, t, pat_span);
                            self.prog.add_pat(TPat::Bind(sym, Some(test)))
                        }
                        None => test,
                    }
                }
                None => self.prog.add_pat(TPat::Wildcard),
            };
            self.push_scope();
            if let TPat::Bind(sym, _) = self.prog.pats[pat.idx()] {
                let name = self.syms.sym(sym).name;
                self.bind_local(name, sym);
                self.bind_given(sym);
            }
            let guard = c.guard.map(|g| self.check_expr(g, self.b.t_boolean));
            let (body, bty) = self.type_expr_adapted(c.body, expected);
            self.pop_scope();
            joined = Some(match joined {
                Some(j) if bty != ERROR && j != ERROR => self.lub(j, bty),
                Some(j) if j != ERROR => j,
                _ => bty,
            });
            typed.push(TCase { pat, guard, body });
        }
        let ty = expected.unwrap_or_else(|| joined.unwrap_or(ERROR));
        let scrutinee = self.prog.add(TExpr::Unit);
        let unit = self.b.t_unit;
        self.prog.set_type(scrutinee, unit);
        let l = self.prog.cases.push_slice(&typed);
        let te = self.prog.add(TExpr::Match(scrutinee, l));
        self.prog.set_type(te, ty);
        self.note_reducible(te, ReducibleSource::SummonFrom { whole: span, cases: written });
        (te, ty)
    }

    /// `summonFrom { case given T => ...; case x: T => ...; case _ => ... }`: the first case
    /// whose type has a given at the call site, or the wildcard.
    fn summon_from(&mut self, arg: ExprId, span: Span, expected: Option<TypeId>) -> (TExprId, TypeId) {
        let cases = self.summon_from_cases(arg);
        if cases.is_empty() {
            self.error(span, "summonFrom takes a pattern-matching anonymous function");
            return (self.prog.add(TExpr::Unit), ERROR);
        }
        let ast = self.cur_ast();
        let clauses = ast.case_list(cases).to_vec();
        let outer_origin = std::mem::replace(&mut self.profile.summon, super::profile::Origin::SummonFrom);
        for c in &clauses {
            let Some((binder, ty)) = self.summon_from_case(c.pat) else { continue };
            let found = match ty {
                Some(ty) => {
                    let t = self.resolve_type(ty);
                    match self.summon_at_site(t, span) {
                        Some((te, given_ty)) => Some((te, given_ty)),
                        None => continue,
                    }
                }
                None => None,
            };
            self.push_scope();
            let mut stmts = Vec::new();
            if let Some(name) = binder {
                let (te, ty) = found.unwrap_or_else(|| (self.prog.add(TExpr::Unit), self.b.t_unit));
                let sym = self.new_local(name, SymKind::Val, ty, span);
                stmts.push(TStmt::Val(sym, te));
                self.bind_local(name, sym);
                if ast.given_binds.contains(&c.pat) {
                    self.syms.sym_mut(sym).mods |= crate::ast::mods::GIVEN;
                    self.bind_given(sym);
                }
            }
            self.profile.summon = outer_origin;
            let (body, bty) = self.type_expr_adapted(c.body, expected);
            self.pop_scope();
            if stmts.is_empty() {
                return (body, bty);
            }
            let l = self.prog.stmts.push_slice(&stmts);
            return (self.prog.add(TExpr::Block(l, body)), bty);
        }
        self.profile.summon = outer_origin;
        let mut msg = "cannot reduce summonFrom with\n patterns :".to_string();
        for (i, c) in clauses.iter().enumerate() {
            let text = self.source_text(self.env.file, ast.pat_spans[c.pat.idx()]);
            msg.push_str(if i == 0 { "  case " } else { "\n             case " });
            msg.push_str(&text);
        }
        self.error(span, msg);
        (self.prog.add(TExpr::Unit), ERROR)
    }

    /// A given for `t` found where the outermost inline call stands, which is where scalac
    /// expands the body and runs its searches.
    pub fn summon_at_site(&mut self, t: TypeId, span: Span) -> Option<(TExprId, TypeId)> {
        self.summon_at_site_telling(t, span).ok()
    }

    /// `summon_at_site`, telling a search that found nothing (`Err(false)`) from an ambiguous
    /// one (`Err(true)`).
    pub fn summon_at_site_telling(&mut self, t: TypeId, span: Span) -> Result<(TExprId, TypeId), bool> {
        use super::profile::Origin;
        let outer = self.profile.summon;
        if outer == Origin::Plain {
            self.profile.summon = Origin::SummonInline;
        }
        let found = match self.inline.site_env.clone() {
            Some(env) => {
                let site_span = self.inline.sites.first().map_or(span, |s| s.span);
                let mut env = (*env).clone();
                // The scopes the expanded bodies open around the call stand nearer than the
                // site's, each with the givens it defines and the imports it holds, as the
                // block that holds them does: `{ given Int = 7; summonInline[Int] }` is 7
                // wherever it expands, `{ import V.*; given Int = 8; summonInline[Int] }`
                // ambiguous where `V` has an implicit `Int`.
                let mut scopes = self.inline.outer_scopes.clone();
                scopes.extend(self.body_scopes());
                // The expanded code is the site's owner's, as scalac's inlined trees are.
                let site_owner = env.frames.last().map_or(0, |f| match f {
                    Frame::Locals { owner, .. } => *owner,
                    Frame::Class(_) => u32::MAX,
                });
                for scope in &scopes {
                    env.frames.push(Frame::Locals { names: Vec::new(), tparams: Vec::new(), givens: scope.givens.clone(), classes: Vec::new(), aliases: Vec::new(), owner: site_owner });
                    let at = env.imports.len();
                    for &imp in &scope.imports {
                        env.push_import(at, imp);
                    }
                }
                // An expansion the search makes (an inline given) stands in those scopes too,
                // which the search's environment holds rather than the body's frames.
                let outer_scopes = std::mem::replace(&mut self.inline.outer_scopes, scopes);
                let body_imports = std::mem::take(&mut self.inline.body_imports);
                self.inline.body_frames.push(env.frames.len());
                // The search stands at the call: what it takes uses the site's imports, as
                // scalac resolves `summonInline` at the expansion's site.
                self.unused.at_site += 1;
                let found = self.with_env(env, |typer| typer.resolve_given_telling(t, site_span));
                self.unused.at_site -= 1;
                self.inline.body_frames.pop();
                self.inline.body_imports = body_imports;
                self.inline.outer_scopes = outer_scopes;
                found
            }
            None => self.resolve_given_telling(t, span),
        };
        self.profile.summon = outer;
        found
    }

    /// A read, in the selected case's right-hand side `body` before it reduces any further, of a
    /// binder an erased scrutinee bound (`unusable` from `mark`), reported at the outermost inline
    /// call: `InlineReducer.reduceInlineMatch`'s `cleanupUnusable` runs on the reduced match, the
    /// case's body still holding its own `inline if` and matches. `originals` names the stored
    /// binders an expansion renamed into these, which a part of the body it left uncopied reads.
    pub(super) fn report_unusable_reads(&mut self, mark: usize, body: TExprId, originals: &[SymId], span: Span) {
        if self.inline.unusable.len() <= mark {
            return;
        }
        let unusable = &self.inline.unusable[mark..];
        let read = self.prog.descendants(body).find_map(|e| match self.prog.expr(e) {
            TExpr::Local(s) if unusable.contains(&s) || originals.contains(&s) => Some(s),
            _ => None,
        });
        if let Some(s) = read {
            let (file, at) = self.inline.sites.first().map_or((self.env.file, span), |site| (site.file, site.span));
            let msg = format!("value {} is unusable because it refers to an erased expression in the selector of an inline match", self.name_str(self.syms.sym(s).name));
            self.diags.error(file, at, msg);
        }
    }

    /// A part the retype path's reduction drops untyped (an `inline if`'s other branch, an
    /// `inline match`'s other cases) under an erased scrutinee's binders: typed aside, what it
    /// reports and makes retracted, and a read of one of them in it reported as one in the case's
    /// body is (`cleanupUnusable` before the reduction of the body).
    fn check_dropped_unusable(&mut self, part: ExprId, span: Span) {
        let mark = self.attempt();
        let diags = self.diags.items.len();
        let (te, _) = self.type_expr(part, None);
        let read = self.prog.descendants(te).find_map(|e| match self.prog.expr(e) {
            TExpr::Local(s) if self.inline.unusable.contains(&s) => Some(s),
            _ => None,
        });
        self.drop_reported_since(diags);
        self.retract(mark);
        if let Some(s) = read {
            let (file, at) = self.inline.sites.first().map_or((self.env.file, span), |site| (site.file, site.span));
            let msg = format!("value {} is unusable because it refers to an erased expression in the selector of an inline match", self.name_str(self.syms.sym(s).name));
            self.diags.error(file, at, msg);
        }
    }

    /// The vals of the binders an erased scrutinee bound for the case reduced (`unusable` from
    /// `mark`), dropped (`cleanupUnusable`).
    pub(super) fn drop_unusable(&mut self, mark: usize, stmts: &mut Vec<TStmt>) {
        if self.inline.unusable.len() <= mark {
            return;
        }
        let unusable = self.inline.unusable.split_off(mark);
        stmts.retain(|st| !matches!(st, TStmt::Val(v, _) if unusable.contains(v)));
    }

    /// Records the erased value `e`, an `erasedValue` the typing made or an expansion copied, for
    /// the check of its unit's end (`report_erased_values`), at the outermost inline call's site
    /// where an expansion made it; not in a body the definition check types, which no tree keeps.
    pub(super) fn note_erased_value(&mut self, e: TExprId, span: Span) {
        if !self.checks_inline_definition() {
            let (file, at) = self.inline.sites.first().map_or((self.env.file, span), |s| (s.file, s.span));
            self.erased_values.push((e, file, at));
        }
    }

    /// Whether `e` calls `erasedValue` (scala-library's a plain method of its package object, the
    /// lean library's the intrinsic's deferred call).
    pub(super) fn calls_erased_value(&mut self, e: TExprId) -> bool {
        match self.prog.expr(e) {
            TExpr::CallStatic(s, _) | TExpr::CallMethod(_, s, _) => {
                self.syms.sym(s).name == crate::names::ERASED_VALUE && self.intrinsic_of(s) == Some(Intrinsic::ErasedValue)
            }
            _ => false,
        }
    }

    /// The values `erasedValue` made inside the scrutinee `scrut` that an `inline match` tests:
    /// no values, no longer recorded as such, and handed back.
    pub(super) fn erased_tested(&mut self, scrut: TExprId) -> Vec<TExprId> {
        if self.erased_values.is_empty() {
            return Vec::new();
        }
        let inside: Vec<TExprId> = self.prog.descendants(scrut).collect();
        let mut tested = Vec::new();
        self.erased_values.retain(|m| {
            let within = inside.contains(&m.0);
            if within {
                tested.push(m.0);
            }
            !within
        });
        tested
    }

    /// The argument of the constructor application `scrut` a field `i` of `c` projects to where the
    /// scrutinee is erased (`InlineReducer.reduceProjection` over a precomputed instance): an
    /// elideable one that holds no erased value, which the field's binding reads instead of the
    /// erased scrutinee, so that `adjustErased` finds no erased reference in it. Another argument
    /// is read through the scrutinee, erased as the scrutinee is.
    pub(super) fn erased_free_projection(&mut self, scrut: TExprId, erased: bool, parts: &[TExprId], c: ClassId, i: usize) -> Option<TExprId> {
        if !erased {
            return None;
        }
        let TExpr::New(k, items) = self.prog.expr(scrut) else { return None };
        let item = *self.prog.expr_list(items).get(i).filter(|_| k == c)?;
        let holds_erased = self.prog.descendants(item).any(|e| parts.contains(&e)) || self.names_erased_value(item);
        (!holds_erased && self.elideable_scrutinee(item)).then_some(item)
    }

    /// Reports the values `erasedValue` made since `mark` that the unit's trees `roots` still
    /// hold: used as values, not tested by an `inline match` (scalac's `Erasure.checkNotErased`).
    pub(super) fn report_erased_values(&mut self, mark: usize, roots: &[TExprId]) {
        if self.erased_values.len() <= mark {
            return;
        }
        let mut made: Vec<(TExprId, FileId, Span)> = self.erased_values.split_off(mark);
        made.sort_by_key(|m| m.0);
        let mut used: Vec<(FileId, Span)> = Vec::new();
        for &root in roots {
            for e in self.prog.descendants(root) {
                if let Ok(i) = made.binary_search_by_key(&e, |m| m.0) {
                    let (_, file, span) = made[i];
                    if !used.contains(&(file, span)) {
                        used.push((file, span));
                    }
                }
            }
        }
        for (file, span) in used {
            self.diags.error(file, span, "method erasedValue is declared as `erased`, but is in fact used");
        }
    }

    pub(super) fn inline_intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        sig: &MethodSig,
        subst: &Subst,
        args: &[TExprId],
        ret_ty: TypeId,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let part = self.part(Part::Intrinsic);
        let r = self.inline_intrinsic_now(intrinsic, sig, subst, args, ret_ty, span, expected);
        self.part_end(part);
        r
    }

    fn inline_intrinsic_now(
        &mut self,
        intrinsic: Intrinsic,
        sig: &MethodSig,
        subst: &Subst,
        args: &[TExprId],
        ret_ty: TypeId,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let targ = |t: &mut Self| {
            let p = sig.tparams.first().copied();
            let t0 = p.map_or(ERROR, |p| t.types.param(p));
            let t0 = t.types.subst(t0, subst);
            let t0 = t.zonk(t0);
            t.solve_if_var(t0)
        };
        let unit = |t: &mut Self| t.prog.add(TExpr::Unit);
        match intrinsic {
            Intrinsic::ErasedValue => {
                let t = targ(self);
                let e = unit(self);
                self.inline.erased = Some(e);
                self.note_erased_value(e, span);
                (e, t)
            }
            Intrinsic::ConstValue => {
                let t = targ(self);
                match self.fold_type(t) {
                    Some(v) => self.constant(v),
                    None => {
                        let msg = format!("{} is not a constant type; cannot take constValue", self.show(t));
                        self.error(span, msg);
                        (unit(self), ERROR)
                    }
                }
            }
            Intrinsic::ConstructorOf => {
                let t = targ(self);
                match self.js_class_value(t) {
                    Some(e) => (e, ret_ty),
                    None => {
                        let msg = format!("not supported yet: js.constructorOf[{}], the class value of a class teq compiles", self.show(t));
                        self.error(span, msg);
                        (unit(self), ERROR)
                    }
                }
            }
            Intrinsic::LinkTimeProperty => {
                let value = args.first().and_then(|&a| self.fold_constant(a));
                let name = match value {
                    Some(LitVal::Str(n)) => self.name_ref(n).to_string(),
                    _ => String::new(),
                };
                match self.link_time_property(&name) {
                    Some(v) => (self.constant(v).0, ret_ty),
                    None => {
                        self.error(span, format!("link-time property not found: '{}'", name));
                        (unit(self), ERROR)
                    }
                }
            }
            Intrinsic::ConstValueOpt => {
                let t = targ(self);
                let some = self.scala_class_named("Some");
                let none = self.scala_class_named("None");
                let (Some(some), Some(none)) = (some, none) else {
                    self.error(span, "Option is missing from the standard library");
                    return (unit(self), ERROR);
                };
                match self.fold_type(t) {
                    Some(v) => {
                        let (lit, _) = self.constant(v);
                        let l = self.prog.list(&[lit]);
                        (self.prog.add(TExpr::New(some, l)), ret_ty)
                    }
                    None => (self.prog.add(TExpr::Module(none)), ret_ty),
                }
            }
            Intrinsic::ConstValueTuple => {
                let t = targ(self);
                let Some(elems) = self.tuple_elements(t) else {
                    let msg = format!("{} is not a tuple type; cannot take constValueTuple", self.show(t));
                    self.error(span, msg);
                    return (unit(self), ERROR);
                };
                let mut items = Vec::with_capacity(elems.len());
                for e in elems {
                    match self.fold_type(e) {
                        Some(v) => items.push(self.constant(v).0),
                        None => {
                            let msg = format!("{} is not a constant type; cannot take constValue", self.show(e));
                            self.error(span, msg);
                            return (unit(self), ERROR);
                        }
                    }
                }
                (self.tuple_value(&items), t)
            }
            Intrinsic::SummonInline => {
                let t = targ(self);
                match self.summon_at_site(t, span) {
                    // The given's own type, which for a synthesized mirror says more than `T`.
                    Some((te, found)) => (te, if self.types.contains_error(found) { t } else { found }),
                    None => {
                        let msg = self.given_ambiguity.take().unwrap_or_else(|| format!("No given instance of type {} was found.", self.show(t)));
                        self.error(span, msg);
                        (unit(self), ERROR)
                    }
                }
            }
            Intrinsic::SummonAll => {
                let t = targ(self);
                let Some(elems) = self.tuple_elements(t) else {
                    let msg = format!("{} is not a tuple type; cannot summonAll", self.show(t));
                    self.error(span, msg);
                    return (unit(self), ERROR);
                };
                let mut items = Vec::with_capacity(elems.len());
                for e in elems {
                    match self.summon_at_site(e, span) {
                        Some((te, _)) => items.push(te),
                        None => {
                            let msg = self.given_ambiguity.take().unwrap_or_else(|| format!("No given instance of type {} was found.", self.show(e)));
                            self.error(span, msg);
                            return (unit(self), ERROR);
                        }
                    }
                }
                (self.tuple_value(&items), t)
            }
            Intrinsic::Error => {
                let arg = args.first().copied();
                match arg.and_then(|a| self.fold_constant(a)) {
                    Some(LitVal::Str(s)) => {
                        let msg = self.name_str(s);
                        // At the outermost call, as scalac reports it (`Inliner` 742 to 757): an
                        // inline given a search inserted stands at the end of the call it completes.
                        let (file, at) = self
                            .inline
                            .sites
                            .first()
                            .map_or((self.env.file, span), |s| (s.file, if s.at_end { Span { start: s.span.end, end: s.span.end } } else { s.span }));
                        let late = self.reports_late();
                        self.diags.error(file, at, msg);
                        if late {
                            self.note_late();
                        }
                    }
                    _ => self.error(span, "A literal string is expected as an argument to `compiletime.error`."),
                }
                (unit(self), NOTHING)
            }
            Intrinsic::Uninitialized => {
                let zero = match expected.map(|t| self.dealias(t)) {
                    Some(t) if t == self.b.t_int => TExpr::Int(0),
                    Some(t) if t == self.b.t_long => TExpr::Long(0),
                    Some(t) if t == self.b.t_double => TExpr::Double(0.0),
                    Some(t) if t == self.b.t_boolean => TExpr::Bool(false),
                    Some(t) if t == self.b.t_char => TExpr::Char(0),
                    Some(t) if t == self.b.t_unit => TExpr::Unit,
                    _ => TExpr::Null,
                };
                (self.prog.add(zero), expected.unwrap_or(ret_ty))
            }
            // `@compileTimeOnly` in scala-library: the marker means something as a given's
            // right-hand side in a trait alone, under its own name (dotty's `Erasure`, 536-543).
            Intrinsic::Deferred => {
                let mut msg = String::from("`deferred` can only be used as the right hand side of a given definition in a trait");
                let text = self.source_text(self.env.file, span);
                let written = text.rsplit('.').next().unwrap_or("").trim();
                if !written.is_empty() && written != "deferred" {
                    msg.push_str(&format!(".\nNote that `deferred` can only be used under its own name when implementing a given in a trait; `{}` is not accepted.", written));
                }
                self.error(span, msg);
                (unit(self), NOTHING)
            }
            Intrinsic::TypeChecks => {
                self.error(span, "not supported yet: scala.compiletime.testing.typeChecks (a type check of a string of code)");
                (unit(self), ERROR)
            }
            // The walk's calls, whose argument is the walked tree (`Worker::code_of_tree`); the
            // retype path's take it as written (`early_intrinsic`).
            Intrinsic::CodeOf if args.len() == 1 => {
                let text = self.code_of_tree(args[0]);
                let r = self.prog.add_str(&text);
                (self.prog.add(TExpr::Str(r)), self.b.t_string)
            }
            Intrinsic::RequireConst if args.len() == 1 => {
                let ty = self.inline.arg_types.iter().find(|&&(a, _)| a == args[0]).map(|&(_, t)| t);
                if self.constant_tree_value(args[0], ty).is_none() {
                    let text = self.code_of_tree(args[0]);
                    let late = self.reports_late();
                    self.error(span, format!("expected a constant value but found: {}", text));
                    if late {
                        self.note_late();
                    }
                }
                (unit(self), self.b.t_unit)
            }
            Intrinsic::SummonFrom | Intrinsic::CodeOf | Intrinsic::RequireConst => {
                self.error(span, format!("{} takes its argument as written", self.method_description(sig_sym(self, sig))));
                (unit(self), ERROR)
            }
        }
    }

    /// A typed tree as scalac's `codeOf` and `requireConst` show it (`Tree.show`), for the shapes
    /// an argument the walk expands has: a constant as its literal (`4`, `10L`, `2.5d`, `'c'`,
    /// `"ab"`), a local or a static member by its name, a selection on its receiver, a primitive
    /// or symbolic operation infix, a unary one as its method (`y.unary_-`), a call with its
    /// arguments, a condition; any other tree as it was written.
    pub(super) fn code_of_tree(&mut self, e: TExprId) -> String {
        if let Some(v) = self.fold_constant(e) {
            return self.shown_literal(v, e);
        }
        let name = |t: &Self, s: SymId| t.name_str(t.syms.sym(s).name);
        match self.prog.expr(e) {
            TExpr::Unit => "()".into(),
            TExpr::Null => "null".into(),
            TExpr::This => "this".into(),
            TExpr::Local(s) | TExpr::Static(s) => name(self, s),
            TExpr::Module(c) => self.name_str(self.syms.class(c).name),
            TExpr::Field(r, s) => format!("{}.{}", self.code_of_operand(r, u8::MAX, false), name(self, s)),
            // The conversion a concatenation renders an operand with is no call of the program's;
            // an explicit `toString` is.
            TExpr::ToStr(a, conv) if conv.is_rendering() => self.code_of_tree(a),
            TExpr::ToStr(a, _) => format!("{}.toString()", self.code_of_operand(a, u8::MAX, false)),
            TExpr::StrConcat(items) => {
                let items: Vec<TExprId> = self.prog.expr_list(items).to_vec();
                let plus = infix_precedence("+");
                let shown: Vec<String> = items.into_iter().enumerate().map(|(i, a)| self.code_of_operand(a, plus, i > 0)).collect();
                shown.join(" + ")
            }
            TExpr::Prim(op, a, b) => {
                let shown = crate::interp::quoted::prim_name(op);
                let p = infix_precedence(shown);
                let (a, b) = (self.code_of_operand(a, p, false), self.code_of_operand(b, p, true));
                format!("{} {} {}", a, shown, b)
            }
            TExpr::Unary(op, a) => format!("{}.{}", self.code_of_operand(a, u8::MAX, false), crate::interp::quoted::unary_name(op)),
            TExpr::CallStatic(f, args) => {
                let items: Vec<TExprId> = self.prog.expr_list(args).to_vec();
                let shown: Vec<String> = items.into_iter().map(|a| self.code_of_tree(a)).collect();
                format!("{}({})", name(self, f), shown.join(", "))
            }
            TExpr::CallMethod(r, m, args) => {
                let items: Vec<TExprId> = self.prog.expr_list(args).to_vec();
                let method = name(self, m);
                let symbolic = method.chars().next().map_or(false, |c| !c.is_alphanumeric() && c != '_');
                if items.len() == 1 && symbolic {
                    let p = infix_precedence(&method);
                    let (recv, arg) = (self.code_of_operand(r, p, false), self.code_of_operand(items[0], p, true));
                    return format!("{} {} {}", recv, method, arg);
                }
                let recv = self.code_of_operand(r, u8::MAX, false);
                if items.is_empty() && method.starts_with("unary_") {
                    return format!("{}.{}", recv, method);
                }
                let shown: Vec<String> = items.into_iter().map(|a| self.code_of_tree(a)).collect();
                format!("{}.{}({})", recv, method, shown.join(", "))
            }
            TExpr::If(c, t, Some(f)) => {
                let (c, t, f) = (self.code_of_tree(c), self.code_of_tree(t), self.code_of_tree(f));
                format!("if {} then {} else {}", c, t, f)
            }
            _ => match self.prog.span_of(e) {
                Some((file, span)) => self.source_text(file, span),
                None => self.code_of_template(e).unwrap_or_else(|| "<tree>".into()),
            },
        }
    }

    /// A member of the std that JavaScript reads through a template of a field of its receiver
    /// (`$0.length` of `String.length`), shown as the selection.
    fn code_of_template(&mut self, e: TExprId) -> Option<String> {
        let TExpr::Js(t, args) = self.prog.expr(e) else { return None };
        let member = self.prog.strings[t.idx()].strip_prefix("$0.")?.to_string();
        let &[recv] = self.prog.expr_list(args) else { return None };
        if !member.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        Some(format!("{}.{}", self.code_of_operand(recv, u8::MAX, false), member))
    }

    /// An operand as `code_of_tree` shows it, in parentheses where it shows as an operation of a
    /// lower precedence than its operator's, or of the same on the right (`"a" + (x + y)`, `s + x
    /// * y`); `outer` is `u8::MAX` for a receiver, where every operation takes them.
    fn code_of_operand(&mut self, e: TExprId, outer: u8, right: bool) -> String {
        let text = self.code_of_tree(e);
        match self.shown_precedence(e) {
            Some(p) if p < outer || (right && p == outer) => format!("({})", text),
            _ => text,
        }
    }

    /// The precedence of `e` where `code_of_tree` shows it as an infix operation, a conditional
    /// the lowest; none for another tree. A rendering conversion shows as its operand.
    fn shown_precedence(&mut self, e: TExprId) -> Option<u8> {
        let mut e = e;
        while let TExpr::ToStr(inner, conv) = self.prog.expr(e) {
            if !conv.is_rendering() {
                break;
            }
            e = inner;
        }
        if self.fold_constant(e).is_some() {
            return None;
        }
        match self.prog.expr(e) {
            TExpr::Prim(op, ..) => Some(infix_precedence(crate::interp::quoted::prim_name(op))),
            TExpr::StrConcat(_) => Some(infix_precedence("+")),
            TExpr::If(..) => Some(0),
            TExpr::CallMethod(_, m, args) if args.len == 1 => {
                let method = self.name_str(self.syms.sym(m).name);
                let symbolic = method.chars().next().map_or(false, |c| !c.is_alphanumeric() && c != '_');
                symbolic.then(|| infix_precedence(&method))
            }
            _ => None,
        }
    }

    /// A constant as scalac shows its literal, of the type of the node `e` it folded from.
    fn shown_literal(&mut self, v: LitVal, e: TExprId) -> String {
        match v {
            LitVal::Int(i) => i.to_string(),
            LitVal::Long(l) => format!("{}L", l),
            LitVal::Double(bits) => {
                let d = f64::from_bits(bits);
                let float = self.prog.type_of(e).map_or(false, |t| t == self.b.t_float);
                let mut text = if float { (d as f32).to_string() } else { d.to_string() };
                if d.is_finite() && !text.contains(['.', 'e', 'E']) {
                    text.push_str(".0");
                }
                format!("{}{}", text, if float { "f" } else { "d" })
            }
            LitVal::Char(c) => format!("'{}'", char::from_u32(c as u32).unwrap_or('?')),
            LitVal::Bool(b) => b.to_string(),
            LitVal::Str(s) => format!("\"{}\"", self.name_ref(s).escape_default()),
        }
    }

    // ---- type-level operations ----

    /// `A *: B *: EmptyTuple` as a type: the tuple one longer than the tail, when the tail is
    /// a tuple type.
    /// `h *: t` as the tuple one longer than `t`, or the resolved operands when `t` is no tuple.
    pub(super) fn tuple_cons_type(&mut self, args: ListRef) -> Result<TypeId, [TypeId; 2]> {
        let [h, t] = *self.cur_ast().ty_list(args) else { unreachable!() };
        let head = self.resolve_type(h);
        let tail = self.resolve_type(t);
        let Some(elems) = self.tuple_elements(tail) else { return Err([head, tail]) };
        let mut all = vec![head];
        all.extend(elems);
        Ok(self.tuple_type(&all))
    }

    /// `1 + 2`, `S[N]`, `"a" + "b"` as types: the operation `op` of `scala.compiletime.ops`,
    /// evaluated on literal types; the operation applied to its operands where one is no
    /// literal yet, which `normalize` evaluates once it is.
    pub(super) fn type_level_op(&mut self, op: ClassId, args: ListRef) -> TypeId {
        let arg_ids: Vec<TyExprId> = self.cur_ast().ty_list(args).to_vec();
        let operands: Vec<TypeId> = arg_ids.iter().map(|&a| self.resolve_type(a)).collect();
        let applied = self.types.class(op, &operands);
        self.normalize(applied)
    }

    /// `h *: t` on a value whose type is a tuple type: the tuple one longer.
    pub(super) fn tuple_cons(&mut self, tail: TExprId, tail_ty: TypeId, lists: &[ArgList], span: Span) -> Option<(TExprId, TypeId)> {
        let elems = self.tuple_elements(tail_ty)?;
        let head = match lists {
            [list] if list.args.len() == 1 && !list.using => list.args[0],
            _ => return None,
        };
        let (h, hty) = match head {
            ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => self.type_expr(e, None),
            ArgSrc::Typed(te, ty) | ArgSrc::Named(_, te, ty) => (te, ty),
            ArgSrc::ForLambda(..) => return None,
        };
        let hty = self.solve_inferred(hty);
        let mut items = vec![h];
        let mut tys = vec![hty];
        let tail_class = self.dealias(tail_ty);
        let fields: Vec<SymId> = match self.types.get(tail_class) {
            Type::Class(c, _) if self.is_tuple_class(c) => self.syms.class(c).ctor_syms.concat(),
            _ => Vec::new(),
        };
        let mark = self.hoisted.len();
        let written = tail;
        let tail = if fields.is_empty() { tail } else { self.hoist(tail, tail_ty, span) };
        for i in 0..fields.len() {
            let t = self.copy_expr(tail);
            items.push(self.tuple_element(t, &fields, &elems, i, span));
            tys.push(elems[i]);
        }
        let te = self.tuple_value(&items);
        let ty = self.tuple_type(&tys);
        let te = self.wrap_hoisted(mark, te);
        if self.capturing() {
            self.capture_builtin_call(te, crate::names::CONS_TUPLE, written, vec![h]);
        }
        Some((te, ty))
    }

    /// The object of `scala.compiletime.ops` (`int`, `string`, `boolean`, `any`) that declares
    /// the class `c`, when it is one of the operations.
    pub(super) fn compiletime_op_module(&mut self, c: ClassId) -> Option<Name> {
        let Owner::Class(module) = self.syms.class(c).owner else { return None };
        let Owner::Package(ops) = self.syms.class(module).owner else { return None };
        let compiletime = self.compiletime_pkg()?;
        if self.syms.pkg(ops).parent != Some(compiletime) {
            return None;
        }
        if self.inline.ops_name.is_none() {
            self.inline.ops_name = Some(self.interner.intern("ops"));
        }
        (Some(self.syms.pkg(ops).name) == self.inline.ops_name).then_some(self.syms.class(module).name)
    }

    pub(super) fn scala_class_named(&mut self, name: &str) -> Option<ClassId> {
        let n = self.interner.intern(name);
        let scala = self.b.scala_pkg;
        match self.pkg_term(scala, n) {
            Some(super::resolve::TermRef::Global(s) | super::resolve::TermRef::ModuleMember(_, s)) => match self.syms.sym(s).kind {
                SymKind::Object(c) => Some(c),
                _ => None,
            },
            Some(super::resolve::TermRef::Class(c)) => Some(c),
            _ => match self.pkg_type(scala, n) {
                Some(super::resolve::TypeRef::Class(c)) => Some(c),
                _ => None,
            },
        }
    }

    /// The element types of a tuple type, `EmptyTuple` included, and of `h *: t` with a known
    /// tail.
    pub fn tuple_elements(&mut self, t: TypeId) -> Option<Vec<TypeId>> {
        let mut out = Vec::new();
        let mut t = t;
        loop {
            // A cons is walked as it stands: reducing it first would walk its tail once per
            // element, which is exponential when the tail is stuck. Anything else is reduced.
            let cons = |typer: &Self, t: TypeId| match typer.types.get(t) {
                Type::Class(c, args) if Some(c) == typer.b.cons_tuple => match *typer.types.items(args) {
                    [h, tail] => Some((h, tail)),
                    _ => None,
                },
                _ => None,
            };
            let (h, tail) = match cons(self, t) {
                Some(pair) => pair,
                None => {
                    let d = self.dealias(t);
                    match cons(self, d) {
                        Some(pair) => pair,
                        None => match self.types.get(d) {
                            Type::Class(c, args) if self.is_tuple_class(c) => {
                                out.extend(self.types.items(args).iter().copied());
                                return Some(out);
                            }
                            Type::Class(c, _) if Some(c) == self.empty_tuple_class() => return Some(out),
                            _ if d == self.b.t_unit => return Some(out),
                            // A tail `t & Tuple` from a quote pattern's binder is the tuple it
                            // intersects, as dotc's `glb` drops the part the other conforms to.
                            Type::Inter(a, b) => match self.intersected_tuple(a, b) {
                                Some(elems) => {
                                    out.extend(elems);
                                    return Some(out);
                                }
                                None => return None,
                            },
                            _ => return None,
                        },
                    }
                }
            };
            out.push(h);
            t = tail;
        }
    }

    /// The elements of `a & b` where one side is a tuple type that conforms to the other.
    fn intersected_tuple(&mut self, a: TypeId, b: TypeId) -> Option<Vec<TypeId>> {
        for (x, y) in [(a, b), (b, a)] {
            let Some(elems) = self.tuple_elements(x) else { continue };
            let mark = self.snapshot();
            if self.is_sub(x, y) {
                return Some(elems);
            }
            self.rollback(mark);
        }
        None
    }

    pub(super) fn tuple_value(&mut self, items: &[TExprId]) -> TExprId {
        if items.is_empty() {
            return self.empty_tuple_value();
        }
        if items.len() > 22 {
            if let Some(te) = self.tuple_xxl_value(items, Span::default()) {
                return te;
            }
        }
        let c = self.tuple_class(items.len());
        let l = self.prog.list(items);
        self.prog.add(TExpr::New(c, l))
    }
}

pub(super) enum TypeOpResult {
    Lit(LitVal),
    Str(String),
}

pub(super) fn eval_type_op(module: &str, op: &str, args: &[LitVal], text: impl Fn(Name) -> String) -> Option<TypeOpResult> {
    use LitVal::*;
    let int = |v: &LitVal| match v {
        Int(i) => Some(*i),
        _ => None,
    };
    let bool = |v: &LitVal| match v {
        Bool(b) => Some(*b),
        _ => None,
    };
    let render = |v: &LitVal| match v {
        Int(i) => i.to_string(),
        Long(l) => l.to_string(),
        Double(d) => f64::from_bits(*d).to_string(),
        Bool(b) => b.to_string(),
        Char(c) => char::from_u32(*c as u32).map_or(String::new(), |c| c.to_string()),
        Str(s) => text(*s),
    };
    let lit = |v: LitVal| Some(TypeOpResult::Lit(v));
    match (module, op, args) {
        ("int", "S", [a]) => lit(Int(int(a)?.checked_add(1)?)),
        ("int", "Abs", [a]) => lit(Int(int(a)?.checked_abs()?)),
        ("int", "Negate", [a]) => lit(Int(int(a)?.checked_neg()?)),
        ("int", "ToString", [a]) | ("any", "ToString", [a]) => Some(TypeOpResult::Str(render(a))),
        ("int", _, [a, b]) => {
            let (x, y) = (int(a)?, int(b)?);
            lit(match op {
                "+" => Int(x.checked_add(y)?),
                "-" => Int(x.checked_sub(y)?),
                "*" => Int(x.checked_mul(y)?),
                "/" => Int(x.checked_div(y)?),
                "%" => Int(x.checked_rem(y)?),
                "<<" => Int(x.wrapping_shl(y as u32)),
                ">>" => Int(x.wrapping_shr(y as u32)),
                ">>>" => Int(((x as u32).wrapping_shr(y as u32)) as i32),
                "^" => Int(x ^ y),
                "BitwiseAnd" => Int(x & y),
                "BitwiseOr" => Int(x | y),
                "Min" => Int(x.min(y)),
                "Max" => Int(x.max(y)),
                "<" => Bool(x < y),
                ">" => Bool(x > y),
                "<=" => Bool(x <= y),
                ">=" => Bool(x >= y),
                _ => return None,
            })
        }
        ("string", "+", [a, b]) => {
            let mut out = render(a);
            crate::text::push_str(&mut out, &render(b));
            Some(TypeOpResult::Str(out))
        }
        ("string", "Length", [Str(s)]) => lit(Int(text(*s).chars().map(crate::text::len_utf16).sum::<usize>() as i32)),
        ("boolean", "!", [a]) => lit(Bool(!bool(a)?)),
        ("boolean", "&&", [a, b]) => lit(Bool(bool(a)? && bool(b)?)),
        ("boolean", "||", [a, b]) => lit(Bool(bool(a)? || bool(b)?)),
        ("boolean", "^", [a, b]) => lit(Bool(bool(a)? ^ bool(b)?)),
        ("any", "==", [a, b]) => lit(Bool(a == b)),
        ("any", "!=", [a, b]) => lit(Bool(a != b)),
        ("any", "IsConst", [_]) => lit(Bool(true)),
        _ => None,
    }
}

/// `TEQ_NO_FOLD` turns the folding by evaluation off, for measuring what it costs.
fn no_fold() -> bool {
    static NO_FOLD: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *NO_FOLD.get_or_init(|| std::env::var_os("TEQ_NO_FOLD").is_some())
}

fn sig_sym(t: &Worker, sig: &MethodSig) -> SymId {
    let _ = sig;
    t.inline.sites.last().map_or(SymId(0), |s| s.callee)
}

/// Literal patterns compare numbers by value, whatever class the typer gave the literal.
pub(super) fn same_number(a: LitVal, b: LitVal) -> bool {
    match (a, b) {
        (LitVal::Int(x), LitVal::Long(y)) | (LitVal::Long(y), LitVal::Int(x)) => x as i64 == y,
        (LitVal::Int(x), LitVal::Double(y)) | (LitVal::Double(y), LitVal::Int(x)) => x as f64 == f64::from_bits(y),
        _ => false,
    }
}

/// scalac folds a binary operation over two constants of one kind, or two numbers.
fn same_tag_or_numeric(x: LitVal, y: LitVal) -> bool {
    let numeric = |v: LitVal| matches!(v, LitVal::Int(_) | LitVal::Long(_) | LitVal::Double(_) | LitVal::Char(_));
    std::mem::discriminant(&x) == std::mem::discriminant(&y) || (numeric(x) && numeric(y))
}

fn fold_prim(op: PrimOp, x: LitVal, y: LitVal) -> Option<LitVal> {
    use LitVal::*;
    use PrimOp::*;
    let as_f64 = |v: LitVal| match v {
        Int(i) => Some(i as f64),
        Long(l) => Some(l as f64),
        Double(d) => Some(f64::from_bits(d)),
        Char(c) => Some(c as f64),
        _ => None,
    };
    let as_i64 = |v: LitVal| match v {
        Int(i) => Some(i as i64),
        Long(l) => Some(l),
        Char(c) => Some(c as i64),
        _ => None,
    };
    Some(match (op, x, y) {
        (IntAdd, Int(a), Int(b)) => Int(a.wrapping_add(b)),
        (IntSub, Int(a), Int(b)) => Int(a.wrapping_sub(b)),
        (IntMul, Int(a), Int(b)) => Int(a.wrapping_mul(b)),
        (IntDiv, Int(a), Int(b)) if b != 0 => Int(a.wrapping_div(b)),
        (IntRem, Int(a), Int(b)) if b != 0 => Int(a.wrapping_rem(b)),
        (IntAnd, Int(a), Int(b)) => Int(a & b),
        (IntOr, Int(a), Int(b)) => Int(a | b),
        (IntXor, Int(a), Int(b)) => Int(a ^ b),
        (IntShl, Int(a), Int(b)) => Int(a.wrapping_shl(b as u32)),
        (IntShr, Int(a), Int(b)) => Int(a.wrapping_shr(b as u32)),
        (IntUshr, Int(a), Int(b)) => Int(((a as u32).wrapping_shr(b as u32)) as i32),
        (LongAdd, a, b) => Long(as_i64(a)?.wrapping_add(as_i64(b)?)),
        (LongSub, a, b) => Long(as_i64(a)?.wrapping_sub(as_i64(b)?)),
        (LongMul, a, b) => Long(as_i64(a)?.wrapping_mul(as_i64(b)?)),
        (LongDiv, a, b) if as_i64(b)? != 0 => Long(as_i64(a)?.wrapping_div(as_i64(b)?)),
        (LongRem, a, b) if as_i64(b)? != 0 => Long(as_i64(a)?.wrapping_rem(as_i64(b)?)),
        (LongAnd, a, b) => Long(as_i64(a)? & as_i64(b)?),
        (LongOr, a, b) => Long(as_i64(a)? | as_i64(b)?),
        (LongXor, a, b) => Long(as_i64(a)? ^ as_i64(b)?),
        (LongShl, a, b) => Long(as_i64(a)?.wrapping_shl(as_i64(b)? as u32)),
        (LongShr, a, b) => Long(as_i64(a)?.wrapping_shr(as_i64(b)? as u32)),
        (LongUshr, a, b) => Long(((as_i64(a)? as u64).wrapping_shr(as_i64(b)? as u32)) as i64),
        (DoubleAdd, a, b) => Double((as_f64(a)? + as_f64(b)?).to_bits()),
        (DoubleSub, a, b) => Double((as_f64(a)? - as_f64(b)?).to_bits()),
        (DoubleMul, a, b) => Double((as_f64(a)? * as_f64(b)?).to_bits()),
        (DoubleDiv, a, b) => Double((as_f64(a)? / as_f64(b)?).to_bits()),
        (DoubleRem, a, b) => Double((as_f64(a)? % as_f64(b)?).to_bits()),
        (Lt, a, b) => Bool(as_f64(a)? < as_f64(b)?),
        (Le, a, b) => Bool(as_f64(a)? <= as_f64(b)?),
        (Gt, a, b) => Bool(as_f64(a)? > as_f64(b)?),
        (Ge, a, b) => Bool(as_f64(a)? >= as_f64(b)?),
        (Eq | RefEq, a, b) => Bool(a == b || same_number(a, b)),
        (Ne | RefNe, a, b) => Bool(!(a == b || same_number(a, b))),
        (BoolAnd | BoolStrictAnd, Bool(a), Bool(b)) => Bool(a && b),
        (BoolOr | BoolStrictOr, Bool(a), Bool(b)) => Bool(a || b),
        (BoolXor, Bool(a), Bool(b)) => Bool(a ^ b),
        _ => return None,
    })
}

/// The precedence of an infix operator, scalac's: by its first character, a letter's the lowest.
fn infix_precedence(op: &str) -> u8 {
    match op.chars().next() {
        Some('|') => 2,
        Some('^') => 3,
        Some('&') => 4,
        Some('=' | '!') => 5,
        Some('<' | '>') => 6,
        Some(':') => 7,
        Some('+' | '-') => 8,
        Some('*' | '/' | '%') => 9,
        Some(c) if c.is_alphanumeric() || c == '_' || c == '$' => 1,
        _ => 10,
    }
}
