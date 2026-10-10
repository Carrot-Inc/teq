//! The bodies of a library: the `Term` tree of `src/tasty/terms.rs` converted into the untyped
//! AST of a pseudo file of its own, so that one typer types a library body as it types a
//! source body. References to definitions of the same file name their symbol directly
//! (`Expr::SymRef`), types come in resolved (`TyExpr::Resolved`) unless they mention a binder
//! of the body itself, and members of other files are selected by name through a path from
//! `_root_`. A class is converted whole, with every member: when the backend reaches it
//! (`compile.rs`) and when the expander asks for the body of one of its inline methods.

use super::super::Worker;
use super::types::{is_using_clause, repeated_element};
use super::{flag_mods, LSym};
use crate::ast::{self, *};
use crate::intern::{FxMap, Name};
use crate::names;
use crate::source::Span;
use crate::symbols::ClassKind;
use crate::symbols::*;
use crate::tasty::tags;
use crate::tasty::terms::{tag_name, Case, ClassDef as TClassDef, MatchKind, Stat, Term, TermDecoder, TermKind};
use crate::tasty::tree::{Addr, Clause, Const, Decoder, DefSig, LambdaKind, TParam, TType};
use crate::tasty::{NameRef, TName, TastyFile};
use crate::tir::*;
use crate::types::*;
use std::sync::Arc;

pub(super) struct Conv {
    tasty: Arc<TastyFile>,
    file: u32,
    pub ast: Ast,
    /// Binders of the body itself, by address: locals, parameters of local methods and
    /// lambdas, pattern variables, members of local classes.
    locals: FxMap<Addr, Name>,
    /// Whether the class converted is a product directory's (`converts_product`).
    product: bool,
    /// For a product's body, the binders in scope where the conversion stands, by name and
    /// address, and the binders that shadow one of them that their scope still names, which are
    /// renamed apart (`unshadowed`).
    scope: Vec<(Name, Addr)>,
    renamed: FxMap<Addr, ()>,
    /// For a product's local class whose parents' calls are being converted, the class's
    /// address and its constructor's parameters by name: `C.this.x` there is the parameter `x`,
    /// as the source has it, the class's `this` not existing yet.
    parent_params: Option<(Addr, Vec<Name>)>,
    /// A product's expansions to replay (`ReaderTables::replay`): the methods of its `INLINED`s
    /// by their addresses, and the trees of their leaves and leaf tests.
    replay_callees: FxMap<Addr, SymId>,
    /// The `INLINED`s whose method the origins name and the reader finds no one of, with it.
    replay_unresolved: FxMap<Addr, String>,
    replay_leaves: FxMap<Addr, ()>,
    replay_tests: FxMap<Addr, ()>,
    /// Type parameters of local methods and type variables of patterns, by address.
    local_types: FxMap<Addr, Name>,
    /// The type variables of quote type patterns (`case '[t]`), types of their case bodies.
    quote_type_vars: FxMap<Addr, ()>,
    /// The `Type[t]` a quote type pattern binds next to its type variable `t` (`f$given1` for
    /// `case '[f]`), by the address of its binder and the variable's name.
    type_givens: FxMap<Addr, Name>,
    /// The binders of the holes of the expression quote pattern under conversion, in the order
    /// its `patternHole`s come: each hole becomes `$name`.
    pattern_holes: std::collections::VecDeque<PatId>,
    /// The enclosing local classes whose `this` a nested local class named, each with the val
    /// its block binds to it.
    self_vals: Vec<(Addr, Name)>,
    /// The type variables of the cases under conversion, the enclosing ones included, whose
    /// guard or body names them, by the address of their `BIND`: these become type variables
    /// the typer binds, the others wildcards.
    named_pattern_vars: FxMap<Addr, ()>,
    /// Those of them the case under conversion binds, which its pattern's ascriptions bind.
    case_pattern_vars: Vec<Addr>,
    /// Parameters of the using clauses of local methods, which a body passes on as givens.
    local_givens: FxMap<Addr, ()>,
    /// A product's local class's written overloads of a var's setter name: the class, the
    /// setter's name and its parameter's class.
    written_setters: FxMap<(Addr, String, String), ()>,
    /// The `Quotes` parameters of the functions scalac wrapped splices in: a reference to one
    /// summons the `Quotes` in scope.
    quotes_params: FxMap<Addr, ()>,
    /// The binders of the polymorphic function types under conversion, with the names their
    /// parameters go by (`[t] => (F[t], t) => R` as a refinement of `PolyFunction`).
    poly_binders: Vec<(Addr, Vec<Name>)>,
    unsupported: Option<String>,
    /// The span every node of the member under conversion gets: its line in the pseudo file's
    /// text, so that a diagnostic names the member.
    span: Span,
    /// The address of the tree under conversion, which the nodes made of it stand for.
    at: Addr,
    /// Per node made, the address of the tree it stands for.
    expr_at: Vec<Addr>,
    ty_at: Vec<Addr>,
    pat_at: Vec<Addr>,
    /// Per node whose `expr_at` is a part of its tree (`NodePlaces::wholes`), the tree's.
    whole_at: Vec<(u32, Addr)>,
    pub text: String,
    /// The defs that stand for entered symbols and classes, joined once the file has its id.
    pub def_syms: Vec<(DefId, SymId)>,
    pub def_classes: Vec<(DefId, ClassId)>,
    /// The inline overrides with the body scalac retained for their dispatch.
    pub retained_bodies: Vec<(SymId, ExprId)>,
    /// The std objects holding the extension methods that the bodies apply as Scala.js
    /// implicit classes, whose members the pseudo file imports.
    pub holder_objects: Vec<ClassId>,
    /// The local methods that a `LAMBDA` of the member closes over, and those met so far, kept
    /// out of their block for the closure.
    lambda_targets: Vec<Addr>,
    lambda_defs: FxMap<Addr, (DefSig, Term)>,
    /// Local classes by address, whose `new` becomes the anonymous class expression.
    anon_defs: FxMap<Addr, DefId>,
    /// The declared types of the block's vals, for `x.T` over a wildcard-typed `x`.
    local_val_types: FxMap<Addr, TType>,
    /// The locals declared with a bounded wildcard, whose values are cast to the type read for
    /// them.
    cast_locals: FxMap<Addr, TyExprId>,
    /// The local classes being converted, by address: `$anon.this.T` names their type member `T`.
    local_classes: Vec<Addr>,
    /// The module classes of the block's local objects, by address, with the objects' names:
    /// the class is the object's singleton type.
    local_modules: FxMap<Addr, Name>,
    /// Whether a type pattern of an `inline match` is being converted, where a `BIND` is a type
    /// variable the case binds to what it matched; elsewhere such a variable is read as `Any`,
    /// since the body is erased and teq has no abstract type for it.
    in_inline_pattern: bool,
    in_inline_match: bool,
    /// The classes whose templates are being converted, outermost first; `C.this` of any but
    /// the innermost is an outer `this`.
    this_chain: Vec<ClassId>,
    /// Under `TEQ_READER_DUMP`, the class converted, as the listing names it.
    dump_class: Option<String>,
    /// What is being converted, as a withheld body's failure names it: `the body of p.C.m`.
    definition: String,
    /// The definitions made, each with the address of the tree it stands for: what places a
    /// product's definitions in their source.
    pub def_at: Vec<(DefId, Addr)>,
}

impl Conv {
    pub(super) fn tasty(&self) -> &TastyFile {
        &self.tasty
    }

    pub(super) fn file(&self) -> u32 {
        self.file
    }


    /// The tables of what the conversion keeps beside the AST.
    fn reader(&mut self) -> &mut crate::ast::ReaderTables {
        self.ast.reader.get_or_insert_with(Default::default)
    }

    fn mark_inferred(&mut self, ty: TyExprId) {
        let at = self.ast.inferred_types.partition_point(|&t| t < ty.0);
        if self.ast.inferred_types.get(at) != Some(&ty.0) {
            self.ast.inferred_types.insert(at, ty.0);
        }
    }

    fn new(tasty: Arc<TastyFile>, file: u32) -> Conv {
        Conv {
            tasty,
            file,
            ast: Ast::new(64),
            locals: FxMap::default(),
            product: false,
            scope: Vec::new(),
            renamed: FxMap::default(),
            parent_params: None,
            replay_callees: FxMap::default(),
            replay_unresolved: FxMap::default(),
            replay_leaves: FxMap::default(),
            replay_tests: FxMap::default(),
            local_types: FxMap::default(),
            quote_type_vars: FxMap::default(),
            type_givens: FxMap::default(),
            pattern_holes: std::collections::VecDeque::new(),
            self_vals: Vec::new(),
            named_pattern_vars: FxMap::default(),
            case_pattern_vars: Vec::new(),
            local_givens: FxMap::default(),
            written_setters: FxMap::default(),
            quotes_params: FxMap::default(),
            poly_binders: Vec::new(),
            unsupported: None,
            span: Span::default(),
            at: 0,
            expr_at: Vec::new(),
            ty_at: Vec::new(),
            pat_at: Vec::new(),
            whole_at: Vec::new(),
            text: String::new(),
            def_syms: Vec::new(),
            def_classes: Vec::new(),
            retained_bodies: Vec::new(),
            holder_objects: Vec::new(),
            lambda_targets: Vec::new(),
            lambda_defs: FxMap::default(),
            anon_defs: FxMap::default(),
            local_val_types: FxMap::default(),
            cast_locals: FxMap::default(),
            local_classes: Vec::new(),
            local_modules: FxMap::default(),
            in_inline_pattern: false,
            in_inline_match: false,
            this_chain: Vec::new(),
            dump_class: None,
            definition: String::new(),
            def_at: Vec::new(),
        }
    }
    fn expr(&mut self, e: Expr) -> ExprId {
        self.expr_at.push(self.at);
        self.ast.add_expr(e, self.span)
    }
    fn ty(&mut self, t: TyExpr) -> TyExprId {
        self.ty_at.push(self.at);
        self.ast.add_ty(t, self.span)
    }
    fn pat(&mut self, p: Pat) -> PatId {
        self.pat_at.push(self.at);
        self.ast.add_pat(p, self.span)
    }
    /// Records that `e` stands for the tree at `at` whole (`NodePlaces::wholes`); an enclosing
    /// tree, recorded later, over an inner one.
    fn whole(&mut self, e: ExprId, at: Addr) {
        self.whole_at.push((e.idx() as u32, at));
    }
    /// Runs `f` with the nodes it makes standing for the tree at `at`.
    fn at<T>(&mut self, at: Addr, f: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.at, at);
        let made = f(self);
        self.at = outer;
        made
    }
    /// Records what the converter has no reading for; the node that stands in its place
    /// reports it when the body is typed. `unsupported` keeps the first reason for the callers
    /// that refuse a whole body, the inline expander.
    fn refuse(&mut self, what: impl Into<String>) -> ExprId {
        let what = what.into();
        if self.unsupported.is_none() {
            self.unsupported = Some(what.clone());
        }
        let s = self.ast.add_str(what);
        self.expr(Expr::Unsupported(s))
    }
    /// Opens a line of the pseudo file for the member `name`; the span of what follows.
    fn line(&mut self, name: &str) -> Span {
        let start = self.text.len() as u32;
        self.text.push_str(name);
        let end = self.text.len() as u32;
        self.text.push('\n');
        self.span = Span { start, end };
        self.span
    }
}

/// An `inline$x` accessor of a class: the class, its template, and the accessor's signature
/// and right-hand side.
struct Accessor {
    class: ClassId,
    def: Arc<TClassDef>,
    sig: DefSig,
    rhs: Option<Term>,
}

fn takes_terms(sig: &DefSig) -> bool {
    sig.clauses.iter().any(|cl| matches!(cl, Clause::Terms(ps) if !ps.is_empty()))
}

enum PredefWrapper {
    Identity,
    ArrayToSeq,
    /// `copyArrayToImmutableIndexedSeq`: a sequence over a copy of the array.
    ArrayCopyToSeq,
}

/// A class of a jar converted into the AST of a pseudo file.
pub struct ConvertedClass {
    pub def: DefId,
    pub ast: Ast,
    pub text: String,
    pub def_syms: Vec<(DefId, SymId)>,
    pub def_classes: Vec<(DefId, ClassId)>,
    pub retained_bodies: Vec<(SymId, ExprId)>,
    pub holder_objects: Vec<ClassId>,
    pub def_at: Vec<(DefId, Addr)>,
}

impl<'a> Worker<'a> {
    /// The class `c` of a jar with its whole template as an AST: the constructor parameters
    /// with their defaults, the parents with their arguments, every member with its body and
    /// the nested classes, each joined to the symbol the loader entered for it. `None` names
    /// what the converter refuses.
    pub(in crate::typer) fn convert_class(&mut self, c: ClassId) -> Result<ConvertedClass, String> {
        self.complete_class(c);
        let lc = self.loaded.as_ref().unwrap().classes[&c];
        let tasty = self.tasty(lc.file);
        let cd = self.decoded_class(lc.file, lc.addr);
        let mut cv = Conv::new(tasty, lc.file);
        cv.product = self.loaded.as_ref().map_or(false, |l| l.cp.is_products(l.file(lc.file).cp));
        if super::declared::dump_path().is_some() {
            cv.dump_class = Some(self.class_path(c));
        }
        cv.at = cd.addr;
        self.prepare_replay(&mut cv);
        let def = self.conv_class_def(&mut cv, &cd, c);
        debug_assert!(cv.expr_at.len() == cv.ast.exprs.len() && cv.ty_at.len() == cv.ast.tys.len() && cv.pat_at.len() == cv.ast.pats.len());
        let mut wholes = std::mem::take(&mut cv.whole_at);
        wholes.reverse();
        wholes.sort_by_key(|&(i, _)| i);
        wholes.dedup_by_key(|&mut (i, _)| i);
        let places = crate::ast::NodePlaces { file: lc.file, exprs: std::mem::take(&mut cv.expr_at), pats: std::mem::take(&mut cv.pat_at), wholes };
        cv.reader().places = places;
        Ok(ConvertedClass { def, ast: cv.ast, text: cv.text, def_syms: cv.def_syms, def_classes: cv.def_classes, retained_bodies: cv.retained_bodies, holder_objects: cv.holder_objects, def_at: cv.def_at })
    }

    /// The template of the class at `addr` of a loaded file, decoded once: a class's conversion
    /// reads its companion's too.
    fn decoded_class(&mut self, file: u32, addr: Addr) -> Arc<TClassDef> {
        if let Some(cd) = self.loaded.as_ref().unwrap().decoded_classes.get(&(file, addr)) {
            return cd.clone();
        }
        let p = self.phase(super::super::profile::Phase::TermDecode);
        let tasty = self.tasty(file);
        let mut decoder = Decoder::new(&tasty);
        let cd = Arc::new(TermDecoder::new(&mut decoder).class_def(addr));
        self.phase_end(p);
        self.loaded_mut().decoded_classes.insert((file, addr), cd.clone());
        cd
    }

    /// Decodes on other threads the templates the conversion of `classes` reads, each one's
    /// root and the root's companion, before the conversions ask: a template is a function of its
    /// file's bytes, so the conversion reads the same whichever thread decoded it. A thread is
    /// started for 64 templates at least, which a start costs as much as (a round of the frontend's
    /// reach decodes 30 to 700); fewer are left to the conversions. How many it decoded.
    pub fn decode_templates_ahead(&mut self, classes: &[ClassId]) -> usize {
        const PER_THREAD: usize = 64;
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        // `TEQ_DECODE_AHEAD=off`, the control for what it saves.
        if *OFF.get_or_init(|| std::env::var_os("TEQ_DECODE_AHEAD").is_some_and(|v| v == "off")) {
            return 0;
        }
        let Some(loaded) = self.loaded.as_ref() else { return 0 };
        let mut wanted: Vec<(u32, Addr)> = Vec::new();
        for &c in classes {
            if !self.is_library_class(c) {
                continue;
            }
            let root = self.conversion_root(c);
            if self.syms.class(root).def.is_some() {
                continue;
            }
            for k in std::iter::once(root).chain(self.syms.class(root).companion) {
                if let Some(lc) = loaded.classes.get(&k) {
                    if !loaded.decoded_classes.contains_key(&(lc.file, lc.addr)) {
                        wanted.push((lc.file, lc.addr));
                    }
                }
            }
        }
        wanted.sort_unstable();
        wanted.dedup();
        let threads = crate::workers().min(wanted.len() / PER_THREAD);
        if threads < 2 {
            return 0;
        }
        let files: FxMap<u32, Arc<TastyFile>> = wanted.iter().map(|&(f, _)| (f, self.tasty(f))).collect();
        let next = std::sync::atomic::AtomicUsize::new(0);
        let work = || {
            let mut done = Vec::new();
            loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(&(file, addr)) = wanted.get(i) else { break };
                let mut decoder = Decoder::new(&files[&file]);
                done.push((i, Arc::new(TermDecoder::new(&mut decoder).class_def(addr))));
            }
            done
        };
        let mut decoded: Vec<(usize, Arc<TClassDef>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..threads)
                .map(|_| {
                    std::thread::Builder::new()
                        .stack_size(1 << 30)
                        .spawn_scoped(scope, || {
                            crate::alloc::enter();
                            work()
                        })
                        .expect("cannot start a decoding thread")
                })
                .collect();
            handles.into_iter().flat_map(|h| h.join().expect("a decoding thread panicked")).collect()
        });
        decoded.sort_unstable_by_key(|&(i, _)| i);
        let n = decoded.len();
        let into = &mut self.loaded_mut().decoded_classes;
        for (i, cd) in decoded {
            into.insert(wanted[i], cd);
        }
        n
    }

    fn conv_name(&mut self, cv: &Conv, n: u32) -> Name {
        let tasty = cv.tasty.clone();
        self.lname(&tasty, tasty.source_name(n))
    }

    fn term_sym(&self, cv: &Conv, addr: Addr) -> Option<SymId> {
        self.file_tables(cv.file).terms.get(&addr).copied()
    }

    /// The symbol at `addr`, completing the classes of the file first where none is entered
    /// yet: a body may name a member of a companion object not completed so far (the
    /// `deriveCache` of tapir's `SchemaMagnoliaDerivation`).
    fn term_sym_completing(&mut self, cv: &Conv, addr: Addr) -> Option<SymId> {
        if let Some(s) = self.term_sym(cv, addr) {
            return Some(s);
        }
        let classes: Vec<ClassId> = self.file_tables(cv.file).classes.values().copied().collect();
        let mut completed = false;
        for c in classes {
            if self.syms.class(c).state() == Completion::NotStarted {
                self.complete_class(c);
                completed = true;
            }
        }
        if completed { self.term_sym(cv, addr) } else { None }
    }

    // ---- classes ----

    /// The template of the entered class `c` as a class definition of the pseudo file.
    fn conv_class_def(&mut self, cv: &mut Conv, cd: &TClassDef, c: ClassId) -> DefId {
        let info = self.syms.class(c);
        let (kind, mods, name) = (info.kind, info.mods, info.name);
        let ast_kind = match kind {
            ClassKind::Object => ast::ClassKind::Object,
            ClassKind::Trait => ast::ClassKind::Trait,
            ClassKind::Enum => ast::ClassKind::Enum,
            ClassKind::EnumCase => ast::ClassKind::EnumCase,
            _ => ast::ClassKind::Class,
        };
        let span = cv.line(&format!("{} {}", kind_word(kind), self.name_ref(name)));
        cv.definition = format!("the parent's call of {}", self.class_path(c));
        cv.this_chain.push(c);
        let ctor_defaults = self.ctor_defaults(cv, c, cd);
        let clauses = self.conv_ctor_clauses(cv, c, cd, &ctor_defaults);
        // A product's parents' arguments see the constructor's parameters as the source has
        // them, plain names, not the members they are in the body.
        let mut scoped = Vec::new();
        let params = if self.converts_product(cv) { cd.template.params.as_slice() } else { &[] };
        let scope_mark = cv.scope.len();
        for p in params {
            if !cv.locals.contains_key(&p.addr) {
                let n = self.conv_name(cv, p.name);
                cv.locals.insert(p.addr, n);
                cv.scope.push((n, p.addr));
                scoped.push(p.addr);
            }
        }
        let parents = self.conv_parents(cv, &cd.template.parents);
        cv.scope.truncate(scope_mark);
        for a in scoped {
            cv.locals.remove(&a);
        }
        let mut member_defaults: FxMap<(Name, usize), (&DefSig, &Term)> = FxMap::default();
        // Every member has its symbol before any body refers to it: a private one is entered here.
        for s in &cd.template.stats {
            match s {
                Stat::Def(sig, rhs) => {
                    if let (Some((owner, index)), Some(rhs)) = (self.default_getter(cv, sig.name), rhs) {
                        member_defaults.insert((owner, index), (sig, rhs));
                    }
                    self.member_sym(cv, c, sig);
                }
                Stat::Val(sig, _) if !sig.mods.flags.has(tags::OBJECT) => {
                    self.member_sym(cv, c, sig);
                }
                _ => {}
            }
        }
        // A closure in the arguments of a secondary constructor's self call has its method at
        // the template's level.
        let mut targets = Vec::new();
        for s in &cd.template.stats {
            if let Stat::Def(_, Some(rhs)) = s {
                targets.extend(lambda_targets(rhs));
            }
        }
        for s in &cd.template.stats {
            if let Stat::Def(sig, Some(rhs)) = s {
                if targets.contains(&sig.addr) {
                    cv.lambda_defs.insert(sig.addr, (sig.clone(), rhs.clone()));
                }
            }
        }
        let retainers: Vec<(NameRef, usize, &DefSig, &Term)> = cd
            .template
            .stats
            .iter()
            .filter_map(|s| match s {
                Stat::Def(sig, Some(rhs)) => Some((cv.tasty.body_retained(sig.name)?, term_param_count(sig), sig, rhs)),
                _ => None,
            })
            .collect();
        let mut body = Vec::with_capacity(cd.template.stats.len());
        for s in &cd.template.stats {
            let at = match s {
                Stat::Val(sig, _) | Stat::Def(sig, _) => sig.addr,
                Stat::Class(cd) => cd.addr,
                Stat::Type(t) => t.addr,
                Stat::Expr(e) => e.at,
                Stat::Import { .. } => cv.at,
            };
            cv.at(at, |cv| self.conv_template_stat(cv, c, s, &member_defaults, &retainers, &mut body));
        }
        if kind == ClassKind::Enum {
            self.conv_enum_values(cv, c);
        }
        // A template statement the producer left out, recorded at the class's definition: the
        // class's construction needs it.
        cv.definition = format!("a template statement of {}", self.class_path(c));
        if let Some(msg) = self.withheld(cv, cd.addr) {
            cv.line("<statement>");
            let s = cv.ast.add_str(msg);
            let e = cv.expr(Expr::Withheld(s));
            body.push(Stmt::Expr(e));
        }
        cv.this_chain.pop();
        let cls = ast::ClassDef { kind: ast_kind, tparams: Vec::new(), clauses, parents, body, exports: ListRef::EMPTY, self_type: None, self_alias: crate::names::EMPTY };
        let d = cv.ast.add_def(Def { name, span, mods: mods & !mods::FINAL | (mods & mods::FINAL), annots: Vec::new(), kind: DefKind::Class(Box::new(cls)) });
        cv.def_at.push((d, cd.addr));
        cv.def_classes.push((d, c));
        d
    }

    /// `<init>$default$N` of the companion object: the default of the N-th constructor
    /// parameter, converted where it stands.
    fn ctor_defaults(&mut self, cv: &mut Conv, c: ClassId, cd: &TClassDef) -> FxMap<usize, ExprId> {
        let mut out = FxMap::default();
        let has_default = self.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter()).any(|p| p.has_default);
        if !has_default {
            return out;
        }
        let Some(companion) = self.syms.class(c).companion else { return out };
        let Some(lc) = self.loaded.as_ref().unwrap().classes.get(&companion).copied() else { return out };
        if lc.file != cv.file {
            return out;
        }
        let stats = if self.syms.class(c).kind == ClassKind::Object || companion == c {
            cd.template.stats.clone()
        } else {
            self.decoded_class(lc.file, lc.addr).template.stats.clone()
        };
        cv.this_chain.push(companion);
        let class_tparams = self.syms.class(c).own_tparams().to_vec();
        for s in &stats {
            if let Stat::Def(sig, Some(rhs)) = s {
                if let Some((owner, index)) = self.default_getter(cv, sig.name) {
                    if owner == names::INIT {
                        // The getter repeats the class's type parameters as its own.
                        for clause in &sig.clauses {
                            if let Clause::Types(ps) = clause {
                                for (tp, &own) in ps.iter().zip(&class_tparams) {
                                    let n = self.syms.tparam(own).name;
                                    cv.local_types.insert(tp.addr, n);
                                }
                            }
                        }
                        let e = self.conv_expr(cv, rhs);
                        out.insert(index, e);
                    }
                }
            }
        }
        cv.this_chain.pop();
        out
    }

    /// `name$default$N`: the method the default belongs to and the 1-based position of the
    /// parameter, for a default getter.
    fn default_getter(&mut self, cv: &Conv, n: u32) -> Option<(Name, usize)> {
        let text = cv.tasty.name(cv.tasty.source_name(n));
        let (owner, index) = text.rsplit_once("$default$")?;
        let index: usize = index.parse().ok()?;
        let owner = if owner == "$lessinit$greater" || owner == "<init>" { names::INIT } else { self.interner.intern(owner) };
        Some((owner, index))
    }

    /// The constructor clauses as `check_class` reads them: one `Param` per parameter of
    /// `ClassInfo::ctor`, with its default.
    fn conv_ctor_clauses(&mut self, cv: &mut Conv, c: ClassId, cd: &TClassDef, defaults: &FxMap<usize, ExprId>) -> Vec<ParamClause> {
        let ctor = self.syms.class(c).ctor.clone();
        let ctor_syms = self.syms.class(c).ctor_syms.clone();
        let mut index = 0;
        let mut clauses = Vec::with_capacity(ctor.len());
        for (cl, syms) in ctor.iter().zip(&ctor_syms) {
            let mut params = Vec::with_capacity(cl.params.len());
            for (p, &sym) in cl.params.iter().zip(syms) {
                index += 1;
                let elem = cv.ty(TyExpr::Resolved(p.ty));
                let ty = if p.repeated { cv.ty(TyExpr::Repeated(elem)) } else { elem };
                let default = defaults.get(&index).copied();
                let m = self.syms.sym(sym).mods;
                params.push(Param { name: p.name, span: cv.span, ty, default, mods: m, annots: ListRef::EMPTY });
            }
            clauses.push(ParamClause { params, is_using: cl.is_using, is_implicit: cl.is_implicit });
        }
        // The addresses of the parameters and their accessors name the parameter symbols.
        let mut i = 0;
        for clause in cd.template.ctor.iter().flat_map(|(sig, _)| sig.clauses.iter()) {
            let Clause::Terms(ps) = clause else { continue };
            for p in ps {
                if let Some(&sym) = ctor_syms.iter().flatten().nth(i) {
                    self.loaded_mut().tables[cv.file as usize].terms.insert(p.addr, sym);
                }
                i += 1;
            }
        }
        for p in &cd.template.params {
            let name = self.conv_name(cv, p.name);
            let sym = ctor.iter().flat_map(|cl| cl.params.iter()).find(|q| q.name == name).map(|q| q.sym);
            if let Some(sym) = sym {
                self.loaded_mut().tables[cv.file as usize].terms.insert(p.addr, sym);
            }
        }
        clauses
    }

    /// Which parameters of a class declare an upper bound below `Any`.
    /// The values of the enum `e`, pickled as vals of its companion whose right-hand sides
    /// create anonymous classes: each becomes a case definition in the enum's own file, as in
    /// source, extending the enum with the arguments its class passes.
    fn conv_enum_values(&mut self, cv: &mut Conv, e: ClassId) {
        let Some(companion) = self.syms.class(e).companion else { return };
        let Some(lc) = self.loaded.as_ref().unwrap().classes.get(&companion).copied() else { return };
        if lc.file != cv.file {
            return;
        }
        let cd = self.decoded_class(lc.file, lc.addr);
        for s in &cd.template.stats {
            if let Stat::Val(sig, rhs) = s {
                if sig.mods.flags.has(tags::ENUM) && sig.mods.flags.has(tags::CASE) {
                    self.conv_enum_value(cv, sig, rhs.as_ref());
                }
            }
        }
    }

    fn conv_enum_value(&mut self, cv: &mut Conv, sig: &DefSig, rhs: Option<&Term>) {
        let Some(sym) = self.term_sym(cv, sig.addr) else { return };
        let SymKind::EnumValue(case) = self.syms.sym(sym).kind else { return };
        let name = self.syms.sym(sym).name;
        let span = cv.line(&format!("case {}", self.name_ref(name)));
        let anon = match rhs.map(|t| &t.kind) {
            Some(TermKind::Block(stats, _)) => stats.iter().find_map(|s| match s {
                Stat::Class(cd) => Some(cd),
                _ => None,
            }),
            _ => None,
        };
        let parents = match anon {
            Some(cd) => self.conv_parents(cv, &cd.template.parents).into_iter().take(1).collect(),
            None => Vec::new(),
        };
        let cls = ast::ClassDef { kind: ast::ClassKind::EnumCase, tparams: Vec::new(), clauses: Vec::new(), parents, body: Vec::new(), exports: ListRef::EMPTY, self_type: None, self_alias: crate::names::EMPTY };
        let d = cv.ast.add_def(Def { name, span, mods: mods::FINAL | mods::CASE, annots: Vec::new(), kind: DefKind::Class(Box::new(cls)) });
        cv.def_at.push((d, sig.addr));
        cv.def_classes.push((d, case));
    }

    /// The parents of a template: constructor calls (`APPLY` of `NEW` under `<init>`) and
    /// plain types, each as the type it names with its argument lists; the builtins (`Object`,
    /// `Any`) are left out, as the loader left them out of `ClassInfo::parents`.
    fn conv_parents(&mut self, cv: &mut Conv, parents: &[Term]) -> Vec<Parent> {
        self.conv_template_parents(cv, parents, false)
    }

    /// A template's parents as pickled (dotty's `TreeUnpickler.readParents`), but a builtin one
    /// (`Object()`), which teq's classes leave implicit as their sources do. An anonymous class's
    /// source names its first parent whatever it is (`new AnyRef { .. }`), so `anonymous` keeps
    /// a builtin one that is all its template has.
    fn conv_template_parents(&mut self, cv: &mut Conv, parents: &[Term], anonymous: bool) -> Vec<Parent> {
        let mut out = Vec::with_capacity(parents.len());
        let mut builtin = None;
        for p in parents {
            // `{ val h$0 = ..; val h$1 = ..; new A(h$0, z = h$1, ..) }`: named arguments out of
            // order, which the call's temporaries hold in the order the source wrote them;
            // read back as the source's arguments. A block
            // that does not read back stays a block, a parent of a shape the converter refuses.
            let mut hoisted: Vec<(Addr, &Term)> = Vec::new();
            let mut p = p;
            if let TermKind::Block(stats, call) = &p.kind {
                if self.converts_product(cv) {
                    let vals: Vec<(Addr, &Term)> = stats
                        .iter()
                        .filter_map(|st| match st {
                            Stat::Val(sig, Some(rhs)) => Some((sig.addr, rhs)),
                            _ => None,
                        })
                        .collect();
                    if !vals.is_empty() && vals.len() == stats.len() && hoisted_order(&call_lists(call), &vals).is_some() {
                        hoisted = vals;
                        p = &**call;
                    }
                }
            }
            let mut head = p;
            let lists = call_lists(p);
            while let TermKind::Apply(f, _) | TermKind::TypeApply(f, ..) = &head.kind {
                head = f;
            }
            let tpt: Option<TType> = match &head.kind {
                TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) if cv.tasty.simple(cv.tasty.source_name(*n)) == Some("<init>") => match &q.kind {
                    TermKind::New(t) => Some(t.clone()),
                    _ => None,
                },
                TermKind::Path(t) => Some(t.clone()),
                TermKind::Typed(_, t, ..) => Some(t.clone()),
                _ => None,
            };
            let Some(tpt) = tpt else {
                cv.refuse("a parent of that shape");
                continue;
            };
            // `new C(args)` with the type arguments left to inference is pickled with the
            // eta-expanded `[X, Y] =>> C[X, Y]`: the bare class, for the typer to infer.
            let (tpt, bare) = match &tpt {
                TType::Lambda { kind: LambdaKind::Type, binder, params, result } => match &**result {
                    TType::Applied(f, args)
                        if args.len() == params.len()
                            && args.iter().enumerate().all(|(i, a)| matches!(a, TType::ParamRef(b, n) if b == binder && *n as usize == i)) =>
                    {
                        ((**f).clone(), true)
                    }
                    _ => (tpt, false),
                },
                _ => (tpt, false),
            };
            // A parent over the enclosing method's type parameters (`new CaseClass[Typeclass,
            // A](...)` in an inline body) names them, for the expansion to fill in.
            let ty = if self.mentions_local_type(cv, &tpt) {
                self.conv_type(cv, &tpt)
            } else {
                let mut cx = super::MapCx::new(cv.file);
                let resolved = if bare { self.map_type_ctor(&mut cx, &tpt) } else { self.map_type(&mut cx, &tpt) };
                let class = match self.types.get(resolved) {
                    Type::Class(k, _) | Type::Ctor(k) => Some(k),
                    _ => None,
                };
                if class.map_or(false, |k| self.syms.class(k).kind == ClassKind::Builtin) {
                    builtin.get_or_insert(resolved);
                    continue;
                }
                cv.ty(TyExpr::Resolved(resolved))
            };
            // A product's parent call passes the constructor's own default getters where the
            // source left the arguments out (`T.$lessinit$greater$default$2`): left out again, the
            // typer supplies them as for the program's own parent (`without_own_defaults`).
            let lists: Vec<&[Term]> = if self.converts_product(cv) && hoisted.is_empty() {
                let mut before = 0;
                lists
                    .into_iter()
                    .map(|list| {
                        let kept = own_defaults_trimmed(cv, list, before);
                        before += list.len();
                        kept
                    })
                    .collect()
            } else {
                lists
            };
            // `C()` passes nothing: the type carries the arguments, an empty list would have the
            // typer infer them from nothing.
            let mut args = Vec::with_capacity(lists.len());
            for list in lists.into_iter().filter(|l| !l.is_empty()) {
                cv.lambda_targets.extend(list.iter().flat_map(lambda_targets));
                cv.lambda_targets.extend(hoisted.iter().flat_map(|(_, rhs)| lambda_targets(rhs)));
                let items: Vec<ExprId> = if hoisted.is_empty() {
                    list.iter().map(|a| self.conv_arg(cv, a)).collect()
                } else {
                    let mut items = Vec::with_capacity(list.len());
                    for a in source_order(list, &hoisted) {
                        let item = match hoisted_arg(a, &hoisted) {
                            Some((None, i)) => self.conv_expr(cv, hoisted[i].1),
                            Some((Some(n), i)) => {
                                let value = self.conv_expr(cv, hoisted[i].1);
                                let name = self.conv_name(cv, n);
                                cv.expr(Expr::NamedArg(name, value))
                            }
                            None => self.conv_arg(cv, a),
                        };
                        items.push(item);
                    }
                    items
                };
                let using = list.iter().all(|a| self.is_given_argument(cv, a));
                args.push((push_list(&mut cv.ast.expr_lists, &items), using));
            }
            out.push(Parent { ty, args });
        }
        if let (true, true, Some(t)) = (anonymous, out.is_empty(), builtin) {
            let ty = cv.ty(TyExpr::Resolved(t));
            out.push(Parent { ty, args: Vec::new() });
        }
        out
    }

    /// A given object of another module's products, which the loader entered as the program's
    /// own given of a class (`enter_product_given_object`): the given's definition, whose value
    /// the check makes as the instance of its class (`check_given`), the class converted from its
    /// module class. `false` where the loader entered the object otherwise.
    fn conv_product_given_object(&mut self, cv: &mut Conv, sig: &DefSig, out: &mut Vec<Stmt>) -> bool {
        let Some(&sym) = self.file_tables(cv.file).terms.get(&sig.addr) else { return false };
        if self.syms.sym(sym).kind != SymKind::Given || self.syms.sym(sym).impl_class.is_none() {
            return false;
        }
        let name = self.syms.sym(sym).name;
        let span = cv.line(&format!("given {}", self.name_ref(name)));
        let ty = self.sig_of(sym).ret;
        let ty = cv.ty(TyExpr::Resolved(ty));
        let given = ast::GivenDef { tparams: Vec::new(), clauses: Vec::new(), ty, alias: None, body: Vec::new(), self_alias: names::EMPTY };
        let m = self.syms.sym(sym).mods;
        let d = cv.ast.add_def(Def { name, span, mods: m, annots: Vec::new(), kind: DefKind::Given(Box::new(given)) });
        cv.def_at.push((d, sig.addr));
        cv.def_syms.push((d, sym));
        out.push(Stmt::Def(d));
        true
    }

    /// The val holding an inner object's instance: `lazy val X = new X()`, the outer instance
    /// going to the constructor as for any inner class.
    fn conv_inner_object_val(&mut self, cv: &mut Conv, c: ClassId, sig: &DefSig, out: &mut Vec<Stmt>) {
        let name = self.conv_name(cv, sig.name);
        let Some(inner) = self.nested_object_of(c, name) else { return };
        let Some(sym) = self.syms.class(inner).inner_object else { return };
        let span = cv.line(&format!("val {}", self.name_ref(name)));
        let class_ty = self.types.class(inner, &[]);
        let t = cv.ty(TyExpr::Resolved(class_ty));
        let init = cv.expr(Expr::New(t, ListRef::EMPTY));
        let d = cv.ast.add_def(Def { name, span, mods: 0, annots: Vec::new(), kind: DefKind::Val { pat: None, ty: Some(t), rhs: Some(init) } });
        cv.def_syms.push((d, sym));
        out.push(Stmt::Def(d));
    }

    fn conv_template_stat(&mut self, cv: &mut Conv, c: ClassId, s: &Stat, defaults: &FxMap<(Name, usize), (&DefSig, &Term)>, retainers: &[(NameRef, usize, &DefSig, &Term)], out: &mut Vec<Stmt>) {
        match s {
            Stat::Val(sig, rhs) => {
                if sig.mods.flags.has(tags::OBJECT) && sig.mods.flags.has(tags::GIVEN) && self.conv_product_given_object(cv, sig, out) {
                    return;
                }
                if sig.mods.flags.has(tags::OBJECT) {
                    self.conv_inner_object_val(cv, c, sig, out);
                    return;
                }
                // An enum value is defined with its enum (`conv_enum_values`).
                if sig.mods.flags.has(tags::ENUM) && sig.mods.flags.has(tags::CASE) {
                    return;
                }
                let Some(sym) = self.member_sym(cv, c, sig) else { return };
                if self.syms.class(c).ctor_syms.iter().flatten().any(|&p| p == sym) {
                    return;
                }
                let name = self.syms.sym(sym).name;
                let span = cv.line(&format!("val {}", self.name_ref(name)));
                cv.definition = format!("the body of {}.{}", self.class_path(c), self.name_ref(name));
                cv.lambda_targets = rhs.as_ref().map(lambda_targets).unwrap_or_default();
                let declared = self.sig_of(sym).ret;
                // A `final val` of a literal type: its value is the type's, and the field is of
                // the widened type; the member keeps the literal type, a constant that a
                // reference folds as scalac folds it.
                let (ty, init) = match self.types.get(declared) {
                    Type::Lit(v) => {
                        let v = self.types.lit_val(v);
                        let widened = self.widened_lit(v);
                        (widened, Some(self.lit_expr(cv, v)))
                    }
                    _ => (declared, rhs.as_ref().map(|r| self.conv_expr(cv, r))),
                };
                let ty = cv.ty(TyExpr::Resolved(ty));
                if sig.ret_inferred {
                    cv.mark_inferred(ty);
                }
                let m = self.syms.sym(sym).mods & (mods::LAZY | mods::MUTABLE | mods::PRIVATE | mods::PROTECTED | mods::OVERRIDE | mods::FINAL | mods::IMPLICIT | mods::GIVEN);
                let d = cv.ast.add_def(Def { name, span, mods: m, annots: Vec::new(), kind: DefKind::Val { pat: None, ty: Some(ty), rhs: init } });
                cv.def_at.push((d, sig.addr));
                cv.def_syms.push((d, sym));
                out.push(Stmt::Def(d));
            }
            Stat::Def(sig, rhs) => {
                // Nothing calls a retainer: its body is its inline method's runtime version.
                if cv.tasty.body_retained(sig.name).is_some() {
                    return;
                }
                // A given class's def of type parameters alone (`given pair2[A, B]: C[A, B] with`),
                // which teq models as the program's: a value, the one instance of its class.
                let terms = sig.clauses.iter().any(|cl| matches!(cl, Clause::Terms(_)));
                if sig.mods.flags.has(tags::GIVEN) && !terms && self.conv_product_given_object(cv, sig, out) {
                    return;
                }
                let Some(sym) = self.member_sym(cv, c, sig) else { return };
                let name = self.syms.sym(sym).name;
                let span = cv.line(&format!("def {}", self.name_ref(name)));
                cv.definition = format!("the body of {}.{}", self.class_path(c), self.name_ref(name));
                // Kept through the conversion of the parameter names below.
                let msig = self.sig_arc(sym);
                // A secondary constructor repeats the class's type parameters as its own, which
                // the loader read as the class's; the body names them by the class's names.
                if name == names::INIT {
                    let class_tparams = self.syms.class(c).own_tparams().to_vec();
                    for clause in &sig.clauses {
                        if let Clause::Types(ps) = clause {
                            for (tp, &own) in ps.iter().zip(&class_tparams) {
                                let n = self.syms.tparam(own).name;
                                cv.local_types.insert(tp.addr, n);
                            }
                        }
                    }
                }
                cv.lambda_targets = rhs.as_ref().map(lambda_targets).unwrap_or_default();
                // A default that is a lambda (`showPathParam: (Int, PathCapture[?]) => String =
                // (index, pc) => ...`) has its method in the default getter's block.
                let method_tparams: Vec<Name> = sig
                    .clauses
                    .iter()
                    .filter_map(|cl| match cl {
                        Clause::Types(ps) => Some(ps.iter().map(|tp| self.conv_name(cv, tp.name)).collect::<Vec<Name>>()),
                        _ => None,
                    })
                    .flatten()
                    .collect();
                let method_terms: Vec<Vec<Name>> = msig.clauses.iter().map(|cl| cl.params.iter().map(|p| p.name).collect()).collect();
                for (getter, d) in defaults.iter().filter(|((n, _), _)| *n == name).map(|(_, d)| *d) {
                    cv.lambda_targets.extend(lambda_targets(d));
                    // The getter repeats the method's type parameters as its own, and the
                    // clauses before the parameter's (`flags: q.reflect.Flags = ..` after
                    // `(using q: Quotes)`): its default names them as the method's.
                    let mut terms = method_terms.iter();
                    for clause in &getter.clauses {
                        match clause {
                            Clause::Types(ps) => {
                                for (tp, &n) in ps.iter().zip(&method_tparams) {
                                    cv.local_types.insert(tp.addr, n);
                                }
                            }
                            Clause::Terms(ps) => {
                                let Some(own) = terms.next() else { break };
                                for (p, &n) in ps.iter().zip(own) {
                                    cv.locals.insert(p.addr, n);
                                }
                            }
                        }
                    }
                }
                for clause in &sig.clauses {
                    if let Clause::Terms(ps) = clause {
                        for p in ps {
                            cv.local_val_types.insert(p.addr, p.ty.clone());
                        }
                    }
                }
                let mut clauses = Vec::with_capacity(msig.clauses.len());
                let mut index = 0;
                let synthetic_apply = name == names::APPLY && sig.mods.flags.has(tags::SYNTHETIC);
                for cl in &msig.clauses {
                    let mut params = Vec::with_capacity(cl.params.len());
                    for p in &cl.params {
                        index += 1;
                        let ty = cv.ty(TyExpr::Resolved(p.ty));
                        // The getters are named for the method alone; only one alternative of
                        // an overloaded name has defaults, and its parameters carry the flag.
                        // A case class companion's synthetic `apply` has the constructor's getters.
                        let own = defaults.get(&(name, index)).or_else(|| synthetic_apply.then(|| defaults.get(&(names::INIT, index))).flatten());
                        let default = own.filter(|_| p.has_default).map(|(_, d)| self.conv_expr(cv, d));
                        let is_inline = self.param_is_inline(cv, sig, index - 1);
                        params.push(Param { name: p.name, span, ty, default, mods: if is_inline { mods::INLINE } else { 0 }, annots: ListRef::EMPTY });
                    }
                    clauses.push(ParamClause { params, is_using: cl.is_using, is_implicit: cl.is_implicit });
                }
                let ret = cv.ty(TyExpr::Resolved(msig.ret));
                if sig.ret_inferred {
                    cv.mark_inferred(ret);
                }
                let ret = Some(ret);
                let body = rhs.as_ref().map(|r| self.conv_expr(cv, r));
                let info = self.syms.sym(sym);
                let m = info.mods & (mods::PRIVATE | mods::PROTECTED | mods::OVERRIDE | mods::FINAL | mods::INLINE | mods::TRANSPARENT | mods::IMPLICIT | mods::GIVEN | mods::ABSTRACT);
                let fun = FunDef {
                    tparams: Vec::new(),
                    clauses,
                    ret,
                    body,
                    ext_tparams: info.ext_tparams,
                    ext_clauses: info.ext_clauses,
                    is_extension: info.is_extension,
                    ext_group: 0,
                };
                let d = cv.ast.add_def(Def { name, span, mods: m, annots: Vec::new(), kind: DefKind::Fun(Box::new(fun)) });
                cv.def_at.push((d, sig.addr));
                cv.def_syms.push((d, sym));
                out.push(Stmt::Def(d));
                if sig.mods.flags.has(tags::INLINE) {
                    let own = cv.tasty.source_name(sig.name);
                    let arity = term_param_count(sig);
                    // Overloads are told apart by the classes their parameters erase to.
                    let erased = self.erased_param_classes(cv, sig);
                    let found = retainers.iter().copied().find(|r| r.0 == own && r.1 == arity && self.erased_param_classes(cv, r.2) == erased);
                    if let Some((_, _, rsig, rhs)) = found {
                        self.conv_retained_body(cv, sym, rsig, rhs);
                    }
                }
            }
            Stat::Class(cd) => {
                let Some(nested) = self.file_tables(cv.file).classes.get(&cd.addr).copied() else { return };
                if !self.loaded.as_ref().unwrap().classes.contains_key(&nested) || self.syms.class(nested).kind == ClassKind::Opaque {
                    return;
                }
                // The loader completes the class before it gets a definition, which would
                // complete it as a source class. The definition stays out of the outer body:
                // the class is checked when the program reaches it.
                self.complete_class(nested);
                self.conv_class_def(cv, cd, nested);
            }
            Stat::Type(_) => {}
            Stat::Import { path, selectors } => {
                if let Some(s) = self.conv_import(cv, path, selectors, false) {
                    out.push(s);
                }
            }
            Stat::Expr(e) => {
                let span = cv.line("<statement>");
                cv.lambda_targets = lambda_targets(e);
                let x = self.conv_expr(cv, e);
                let _ = span;
                out.push(Stmt::Expr(x));
            }
        }
    }

    /// The classes a method's parameters erase to, an array's with its elements' all the way
    /// down (`int[][]` and `String[][]` are two erasures).
    fn erased_param_classes(&mut self, cv: &Conv, sig: &DefSig) -> Vec<Vec<Option<ClassId>>> {
        let params: Vec<TType> = sig
            .clauses
            .iter()
            .filter_map(|c| if let Clause::Terms(ps) = c { Some(ps.iter().map(|p| p.ty.clone())) } else { None })
            .flatten()
            .collect();
        let mut cx = super::MapCx::new(cv.file);
        params
            .iter()
            .map(|t| {
                let mut ty = self.map_type(&mut cx, t);
                let mut erased = Vec::new();
                loop {
                    erased.push(self.class_of(ty));
                    match self.types.get(ty) {
                        Type::Class(c, args) if c == self.b.array && erased.len() < 32 => match self.types.items(args).first() {
                            Some(&e) => ty = e,
                            None => break,
                        },
                        _ => break,
                    }
                }
                erased
            })
            .collect()
    }

    /// The body of `m$retainedBody` as the runtime version of the inline override `sym`: its
    /// parameters are `sym`'s, by name.
    fn conv_retained_body(&mut self, cv: &mut Conv, sym: SymId, sig: &DefSig, rhs: &Term) {
        for clause in &sig.clauses {
            if let Clause::Terms(ps) = clause {
                for p in ps {
                    let n = self.conv_name(cv, p.name);
                    cv.locals.insert(p.addr, n);
                    cv.local_val_types.insert(p.addr, p.ty.clone());
                }
            }
        }
        cv.lambda_targets = lambda_targets(rhs);
        let body = self.conv_expr(cv, rhs);
        cv.retained_bodies.push((sym, body));
    }

    fn widened_lit(&self, v: LitVal) -> TypeId {
        match v {
            LitVal::Int(_) => self.b.t_int,
            LitVal::Long(_) => self.b.t_long,
            LitVal::Double(_) => self.b.t_double,
            LitVal::Char(_) => self.b.t_char,
            LitVal::Bool(_) => self.b.t_boolean,
            LitVal::Str(_) => self.b.t_string,
        }
    }

    fn lit_expr(&mut self, cv: &mut Conv, v: LitVal) -> ExprId {
        let e = match v {
            LitVal::Int(i) => Expr::IntLit(i as i64),
            LitVal::Long(l) => Expr::LongLit(l),
            LitVal::Double(bits) => Expr::DoubleLit(f64::from_bits(bits)),
            LitVal::Char(c) => Expr::CharLit(c as u32),
            LitVal::Bool(b) => Expr::BoolLit(b),
            LitVal::Str(n) => {
                let text = self.name_str(n);
                let s = cv.ast.add_str(text);
                Expr::StringLit(s)
            }
        };
        cv.expr(e)
    }

    fn param_is_inline(&self, _cv: &Conv, sig: &DefSig, index: usize) -> bool {
        sig.clauses
            .iter()
            .filter_map(|c| match c {
                Clause::Terms(ps) => Some(ps.iter()),
                Clause::Types(_) => None,
            })
            .flatten()
            .nth(index)
            .map_or(false, |p| p.flags.has(tags::INLINE))
    }

    /// The symbol of a member of the template: the one the loader entered, or a private
    /// member entered now, which only the class's own bodies reach. Synthetic members are
    /// left out, as the loader left them out.
    fn member_sym(&mut self, cv: &mut Conv, c: ClassId, sig: &DefSig) -> Option<SymId> {
        if let Some(sym) = self.term_sym(cv, sig.addr) {
            return matches!(self.syms.sym(sym).kind, SymKind::Val | SymKind::Var | SymKind::Def | SymKind::Given).then_some(sym);
        }
        let f = sig.mods.flags;
        let pattern_field = sig.tag == tags::VALDEF && super::is_pattern_field_name(Some(&cv.tasty.name(cv.tasty.source_name(sig.name))));
        if (f.has(tags::SYNTHETIC) && !pattern_field) || f.has(tags::ARTIFACT) || !f.has(tags::PRIVATE) || sig.mods.within.is_some() {
            return None;
        }
        let name = self.conv_name(cv, sig.name);
        let file_id = self.syms.class(c).file;
        let kind = if sig.tag == tags::DEFDEF {
            SymKind::Def
        } else if f.has(tags::MUTABLE) {
            SymKind::Var
        } else {
            SymKind::Val
        };
        let m = flag_mods(f) | mods::PRIVATE;
        let sym = self.syms.new_sym(name, kind, m, Owner::Class(c), file_id, None, Span::default());
        self.loaded_mut().syms.insert(sym, LSym { file: cv.file, addr: sig.addr, conversion: false });
        self.loaded_mut().tables[cv.file as usize].terms.insert(sig.addr, sym);
        self.register_term(Owner::Class(c), sym);
        Some(sym)
    }

    // ---- types ----

    fn conv_type(&mut self, cv: &mut Conv, t: &TType) -> TyExprId {
        if let Some(captured) = self.opaque_wildcard_capture(cv, t) {
            return cv.ty(TyExpr::Resolved(captured));
        }
        if !self.mentions_local_type(cv, t) {
            let mut cx = super::MapCx::new(cv.file);
            let resolved = self.map_type(&mut cx, t);
            return cv.ty(TyExpr::Resolved(resolved));
        }
        match t {
            // `$anon.this.T`: a type member of the local class being converted, by its name.
            TType::TypeRef(prefix, n) if self.this_of_local_class(cv, prefix) => {
                let name = self.conv_name(cv, *n);
                cv.ty(TyExpr::Name(name))
            }
            // `$anon.this.type` of the local class being converted is `this.type` there.
            TType::This(_) if self.this_of_local_class(cv, t) => {
                let this = cv.ty(TyExpr::Name(names::THIS));
                cv.ty(TyExpr::Singleton(this))
            }
            // `c.Start` for a local `c`: the member type of a path the typer reads.
            // `C.this.m.type`, the singleton of a member: its declared type.
            TType::TermRef(prefix, n) if matches!(&**prefix, TType::This(_)) => {
                let TType::This(inner) = &**prefix else { unreachable!() };
                let name = self.conv_name(cv, *n);
                let member = self.this_class_of(cv, inner).and_then(|c| {
                    self.complete_class(c);
                    let this_ty = self.syms.this_type(c);
                    self.find_member(this_ty, name).map(|(s, owner_ty)| (s, owner_ty))
                });
                match member {
                    // A val's singleton is the path (`WithQuotes[B, q.type]` for `using val q`).
                    Some((s, _)) if self.syms.sym(s).kind == SymKind::Val && !self.syms.sym(s).by_name => {
                        let mut cx = super::MapCx::new(cv.file);
                        let mapped = self.map_type(&mut cx, t);
                        cv.ty(TyExpr::Resolved(mapped))
                    }
                    Some((s, owner_ty)) => {
                        let ret = self.sig_of(s).ret;
                        let subst = self.owner_subst(owner_ty);
                        let ty = self.types.subst(ret, &subst);
                        let ty = self.widen_path(ty);
                        cv.ty(TyExpr::Resolved(ty))
                    }
                    None => {
                        cv.refuse(format!("the singleton type of member {}", self.name_ref(name)));
                        cv.ty(TyExpr::Error)
                    }
                }
            }
            // `x.T` for a local `x: C[? <: U]` and the parameter `T` of `C`: scalac's capture of
            // the wildcard, read as its bound. A type member of `C` is selected as any other.
            TType::TypeRef(prefix, n) if matches!(&**prefix, TType::LocalTerm(addr, _) if self.wildcard_capture(cv, *addr).is_some() && self.names_class_param(cv, *addr, *n)) => {
                let TType::LocalTerm(addr, _) = &**prefix else { unreachable!() };
                let bound = self.wildcard_capture(cv, *addr).unwrap();
                self.conv_capture(cv, &bound)
            }
            TType::TypeRef(prefix, n) if self.path_mentions_local(cv, prefix) => {
                let path = self.conv_type_path(cv, prefix);
                let name = self.conv_name(cv, *n);
                cv.ty(TyExpr::Select(path, name))
            }
            // `C.this.x.type` for an enclosing class `C` of the body: the loader's path.
            TType::TermRef(prefix, _) if matches!(&**prefix, TType::This(_)) && !self.this_of_local_class(cv, prefix) => {
                let mut cx = super::MapCx::new(cv.file);
                let mapped = self.map_type(&mut cx, t);
                cv.ty(TyExpr::Resolved(mapped))
            }
            TType::LocalTerm(..) | TType::TermRef(..) => {
                let path = self.conv_type_path(cv, t);
                cv.ty(TyExpr::Singleton(path))
            }
            // `sub.SType` for a local `sub: C[? <: U]`: the wildcard's bound, as for `x.T`.
            TType::LocalType(addr, Some(prefix)) if matches!(&**prefix, TType::LocalTerm(a, _) if self.wildcard_capture(cv, *a).is_some() && self.local_names_class_param(cv, *a, *addr)) => {
                let TType::LocalTerm(a, _) = &**prefix else { unreachable!() };
                let bound = self.wildcard_capture(cv, *a).unwrap();
                self.conv_capture(cv, &bound)
            }
            // `$anon.this.Res` for an inherited type member named by its definition: the
            // name resolves in the local class, through the members it overrides.
            TType::LocalType(addr, Some(prefix)) if self.this_of_local_class(cv, prefix) => {
                let tasty = cv.tasty.clone();
                match Decoder::new(&tasty).name_at(*addr) {
                    Some(n) => {
                        let name = self.lname(&tasty, tasty.source_name(n));
                        cv.ty(TyExpr::Name(name))
                    }
                    None => {
                        cv.refuse("a type member without a name");
                        cv.ty(TyExpr::Error)
                    }
                }
            }
            // `c.Start` for a local `c`, the member named by its definition.
            TType::LocalType(addr, Some(prefix)) if self.path_mentions_local(cv, prefix) => {
                let path = self.conv_type_path(cv, prefix);
                let tasty = cv.tasty.clone();
                let name = match Decoder::new(&tasty).name_at(*addr) {
                    Some(n) => self.lname(&tasty, tasty.source_name(n)),
                    None => {
                        cv.refuse("a type member without a name");
                        return cv.ty(TyExpr::Error);
                    }
                };
                cv.ty(TyExpr::Select(path, name))
            }
            // `FlatMap[B] { type Start = c.Start }`: the refinement's type members over the
            // names of the body.
            // `PolyFunction { def apply[t](x: F[t], y: t): R }` over the body's names: the
            // polymorphic function type `[t] => (F[t], t) => R`.
            TType::Refined(parent, members) if self.is_poly_function_parent(cv, parent) => {
                let [(_, Some(TType::Lambda { kind: LambdaKind::Poly, binder, params, result }))] = members.as_slice() else {
                    cv.refuse("a PolyFunction refinement of that shape");
                    return cv.ty(TyExpr::Error);
                };
                let TType::Lambda { kind: LambdaKind::Method, params: term_params, result: ret, .. } = &**result else {
                    cv.refuse("a PolyFunction refinement of that shape");
                    return cv.ty(TyExpr::Error);
                };
                let names: Vec<Name> = params.iter().map(|p| self.conv_name(cv, p.name)).collect();
                cv.poly_binders.push((*binder, names.clone()));
                let ps: Vec<TyExprId> = term_params.iter().map(|p| self.conv_type(cv, &p.info)).collect();
                let r = self.conv_type(cv, ret);
                let bounds = self.conv_lambda_bounds(cv, params);
                cv.poly_binders.pop();
                let pl = push_list(&mut cv.ast.ty_lists, &ps);
                let fun = cv.ty(if is_using_clause(term_params) { TyExpr::CtxFun(pl, r) } else { TyExpr::Fun(pl, r) });
                let nl = push_list(&mut cv.ast.name_lists, &names);
                let id = cv.ty(TyExpr::PolyFun(nl, fun));
                if let Some(bl) = bounds {
                    cv.ast.lambda_bounds.push((id, bl));
                }
                id
            }
            // `[A] =>> Getter[x.Underlying, A]` over a binder of the body (chimney's
            // `Existential[Getter[fallback.Underlying, *]]`): the lambda's parameters by name,
            // its body over the body's names.
            TType::Lambda { kind: LambdaKind::Type, binder, params, result } => {
                let names: Vec<Name> = params.iter().map(|p| self.conv_name(cv, p.name)).collect();
                cv.poly_binders.push((*binder, names.clone()));
                let body = self.conv_type(cv, result);
                cv.poly_binders.pop();
                let nl = push_list(&mut cv.ast.name_lists, &names);
                cv.ty(TyExpr::Lambda(nl, body))
            }
            TType::ParamRef(binder, n) => match cv.poly_binders.iter().rev().find(|(b, _)| b == binder).and_then(|(_, names)| names.get(*n as usize).copied()) {
                Some(name) => cv.ty(TyExpr::Name(name)),
                None => {
                    cv.refuse("a parameter of a lambda type outside its binder");
                    cv.ty(TyExpr::Error)
                }
            },
            TType::Refined(parent, members) => {
                let p = self.conv_type(cv, parent);
                let mut defs = Vec::with_capacity(members.len());
                for (n, info) in members {
                    let name = self.conv_name(cv, *n);
                    let kind = match info {
                        Some(TType::Alias(t)) | Some(TType::BoundedAlias(_, t)) => {
                            let rhs = self.conv_type(cv, t);
                            DefKind::TypeAlias { tparams: Vec::new(), rhs: Some(rhs), lower: None, upper: None }
                        }
                        Some(TType::Bounds(lo, hi)) => {
                            let (l, h) = (self.conv_type(cv, lo), self.conv_type(cv, hi));
                            DefKind::TypeAlias { tparams: Vec::new(), rhs: None, lower: Some(l), upper: Some(h) }
                        }
                        // `Inspector { val qctx: qctx.type }`: a val refined over a path of the body.
                        Some(t) if !matches!(t, TType::Lambda { .. } | TType::ByName(_)) => {
                            let ty = self.conv_type(cv, t);
                            DefKind::Val { pat: None, ty: Some(ty), rhs: None }
                        }
                        _ => {
                            cv.refuse("a refinement with a term member over a local");
                            return cv.ty(TyExpr::Error);
                        }
                    };
                    defs.push(cv.ast.add_def(Def { name, span: cv.span, mods: 0, annots: Vec::new(), kind }));
                }
                let l = push_list(&mut cv.ast.def_lists, &defs);
                cv.ty(TyExpr::Refined(p, l))
            }
            TType::LocalType(addr, _) if cv.local_modules.contains_key(addr) => {
                let path = cv.ty(TyExpr::Name(cv.local_modules[addr]));
                cv.ty(TyExpr::Singleton(path))
            }
            TType::LocalType(addr, _) => match cv.local_types.get(addr).copied() {
                // A type variable of a quote type pattern is a type of the case body.
                Some(name) if cv.quote_type_vars.contains_key(addr) => cv.ty(TyExpr::Name(name)),
                _ if cv.named_pattern_vars.contains_key(addr) => {
                    let name = self.pattern_type_var(cv, *addr);
                    cv.ty(TyExpr::TypeVar(name))
                }
                Some(_) if Decoder::new(&cv.tasty).tag_at(*addr) == tags::BIND && !cv.in_inline_match => cv.ty(TyExpr::Resolved(ANY)),
                Some(name) => cv.ty(TyExpr::Name(name)),
                None if !cv.in_inline_pattern => cv.ty(TyExpr::Resolved(ANY)),
                None => {
                    let name = self.pattern_type_var(cv, *addr);
                    cv.ty(TyExpr::TypeVar(name))
                }
            },
            TType::Applied(f, args) => {
                let mut variances = Vec::new();
                let head = if self.mentions_local_type(cv, f) {
                    self.conv_type(cv, f)
                } else {
                    let mut cx = super::MapCx::new(cv.file);
                    let ctor = self.map_type_ctor(&mut cx, f);
                    variances = self.ctor_variances(ctor);
                    cv.ty(TyExpr::Resolved(ctor))
                };
                let items: Vec<TyExprId> = args
                    .iter()
                    .enumerate()
                    .map(|(i, a)| match a {
                        TType::Bounds(lo, hi) if self.mentions_local_type(cv, lo) || self.mentions_local_type(cv, hi) => {
                            match variances.get(i).copied().unwrap_or(0) {
                                1 => self.conv_type(cv, hi),
                                -1 => self.conv_type(cv, lo),
                                _ => {
                                    let (l, h) = (self.conv_type(cv, lo), self.conv_type(cv, hi));
                                    cv.ty(TyExpr::BoundedWildcard(l, h))
                                }
                            }
                        }
                        TType::Bounds(lo, hi) => {
                            let mut cx = super::MapCx::new(cv.file);
                            let t = self.wildcard_arg(&mut cx, variances.get(i).copied().unwrap_or(0), lo, hi);
                            cv.ty(TyExpr::Resolved(t))
                        }
                        other => self.conv_type_arg(cv, other),
                    })
                    .collect();
                let l = push_list(&mut cv.ast.ty_lists, &items);
                cv.ty(TyExpr::Apply(head, l))
            }
            // `n.type & T` on a local: the flow typing of a cast, which the `T` alone carries.
            TType::And(a, b) if self.is_local_singleton(cv, a) => self.conv_type(cv, b),
            TType::And(a, b) if self.is_local_singleton(cv, b) => self.conv_type(cv, a),
            TType::And(a, b) => {
                let (x, y) = (self.conv_type(cv, a), self.conv_type(cv, b));
                cv.ty(TyExpr::Inter(x, y))
            }
            TType::Or(a, b) => {
                let (x, y) = (self.conv_type(cv, a), self.conv_type(cv, b));
                cv.ty(TyExpr::Union(x, y))
            }
            TType::ByName(inner) => {
                let i = self.conv_type(cv, inner);
                cv.ty(TyExpr::ByName(i))
            }
            TType::Annotated(inner, _) | TType::Flexible(inner) => self.conv_type(cv, inner),
            _ => {
                cv.refuse("a type over a binder of the body");
                cv.ty(TyExpr::Error)
            }
        }
    }

    /// A type argument: a bare class stays a constructor, which a higher-kinded parameter
    /// takes and the typer applies otherwise.
    fn conv_type_arg(&mut self, cv: &mut Conv, a: &TType) -> TyExprId {
        if let Some(captured) = self.opaque_wildcard_capture(cv, a) {
            return cv.ty(TyExpr::Resolved(captured));
        }
        if self.mentions_local_type(cv, a) {
            return self.conv_type(cv, a);
        }
        let mut cx = super::MapCx::new(cv.file);
        let mapped = self.map_type_ctor(&mut cx, a);
        cv.ty(TyExpr::Resolved(mapped))
    }

    /// A `BIND` in a type pattern is a type variable the case binds: named on first sight.
    fn pattern_type_var(&mut self, cv: &mut Conv, addr: Addr) -> Name {
        let tasty = cv.tasty.clone();
        let decoder = Decoder::new(&tasty);
        let name = match decoder.name_at(addr) {
            Some(n) => self.lname(&tasty, tasty.source_name(n)),
            None => self.interner.intern(&format!("t${}", addr)),
        };
        cv.local_types.insert(addr, name);
        name
    }

    fn mentions_named_var(&self, cv: &Conv, t: &TType) -> bool {
        cv.case_pattern_vars.iter().any(|&addr| t.mentions_local(addr))
    }

    fn is_poly_function_parent(&self, cv: &Conv, parent: &TType) -> bool {
        matches!(parent, TType::TypeRef(_, n) if cv.tasty.simple(*n) == Some("PolyFunction"))
    }

    fn this_of_local_class(&self, cv: &Conv, prefix: &TType) -> bool {
        matches!(prefix, TType::This(inner) if matches!(&**inner, TType::LocalType(addr, _) if cv.local_classes.contains(addr)))
    }

    fn is_local_singleton(&self, cv: &Conv, t: &TType) -> bool {
        matches!(t, TType::LocalTerm(..) | TType::TermRef(..)) && self.path_mentions_local(cv, t)
    }

    /// Whether a path names a local of the body, whose type members and singleton type the
    /// loader cannot map and the typer spells out.
    fn path_mentions_local(&self, cv: &Conv, t: &TType) -> bool {
        match t {
            TType::LocalTerm(addr, prefix) => cv.locals.contains_key(addr) || prefix.as_deref().map_or(false, |p| self.path_mentions_local(cv, p)),
            TType::TermRef(prefix, _) => self.path_mentions_local(cv, prefix),
            _ => false,
        }
    }

    /// The capture of a wildcard read as its bound; an unbounded one stays the wildcard, so
    /// that `Remove[r.A](r)` for an `r: Ref[?]` passes `r` as the `Ref[?]` it is.
    fn conv_capture(&mut self, cv: &mut Conv, bound: &TType) -> TyExprId {
        if matches!(bound, TType::TypeRef(_, n) if cv.tasty.simple(*n) == Some("Any")) {
            return cv.ty(TyExpr::Resolved(WILD));
        }
        self.conv_type(cv, bound)
    }

    /// A path as a type expression: `c` and `c.x` over the names of the body.
    fn conv_type_path(&mut self, cv: &mut Conv, t: &TType) -> TyExprId {
        match t {
            TType::LocalTerm(addr, prefix) => match (cv.locals.get(addr).copied(), prefix.as_deref()) {
                (Some(name), _) => cv.ty(TyExpr::Name(name)),
                // A member of a class of the same file selected on a path of the body.
                (None, Some(p)) if self.path_mentions_local(cv, p) => {
                    let tasty = cv.tasty.clone();
                    let Some(n) = Decoder::new(&tasty).name_at(*addr) else {
                        cv.refuse(format!("a path over the definition at {}", addr));
                        return cv.ty(TyExpr::Error);
                    };
                    let q = self.conv_type_path(cv, p);
                    let name = self.lname(&tasty, tasty.source_name(n));
                    cv.ty(TyExpr::Select(q, name))
                }
                _ => {
                    cv.refuse(format!("a path over the definition at {}", addr));
                    cv.ty(TyExpr::Error)
                }
            },
            TType::TermRef(prefix, n) => {
                let p = self.conv_type_path(cv, prefix);
                let name = self.conv_name(cv, *n);
                cv.ty(TyExpr::Select(p, name))
            }
            _ => {
                cv.refuse("a path of that shape in a type");
                cv.ty(TyExpr::Error)
            }
        }
    }

    /// The declared type of a local val with its bounded wildcard arguments read as their upper
    /// bounds (`Array[? <: AnyRef | Null]` as `Array[AnyRef | Null]`): what scalac captures, the
    /// body is typed against. None where the type has no such argument.
    fn local_val_type(&self, cv: &Conv, t: &TType) -> Option<TType> {
        let TType::Applied(f, args) = t else { return None };
        let bounded = |a: &TType| matches!(a, TType::Bounds(_, hi) if !matches!(&**hi, TType::TypeRef(_, n) if matches!(cv.tasty.simple(*n), Some("Any" | "AnyKind"))));
        if !args.iter().any(bounded) {
            return None;
        }
        let args: Vec<TType> = args
            .iter()
            .map(|a| match a {
                TType::Bounds(_, hi) if bounded(a) => (**hi).clone(),
                other => other.clone(),
            })
            .collect();
        Some(TType::Applied(f.clone(), args))
    }

    fn cast_to(&mut self, cv: &mut Conv, e: ExprId, ty: TyExprId) -> ExprId {
        let cast = cv.expr(Expr::Select(e, names::AS_INSTANCE_OF));
        let l = push_list(&mut cv.ast.ty_lists, &[ty]);
        cv.expr(Expr::TypeApply(cast, l))
    }

    /// Whether `n` names a type parameter of the class the local `addr` is declared with,
    /// rather than a type member of it.
    fn names_class_param(&mut self, cv: &Conv, addr: Addr, n: u32) -> bool {
        let Some(TType::Applied(f, _)) = cv.local_val_types.get(&addr) else { return false };
        let f = f.clone();
        let mut cx = super::MapCx::new(cv.file);
        let ctor = self.map_type_ctor(&mut cx, &f);
        let Type::Ctor(c) = self.types.get(ctor) else { return false };
        let name = self.interner.intern(&cv.tasty.name(cv.tasty.source_name(n)));
        self.syms.class(c).tparams.iter().any(|&p| self.syms.tparam(p).name == name)
    }

    /// `names_class_param` for a type parameter defined in the file, named by its address.
    fn local_names_class_param(&mut self, cv: &Conv, local: Addr, param: Addr) -> bool {
        let tasty = cv.tasty.clone();
        let Some(n) = Decoder::new(&tasty).name_at(param) else { return false };
        let name = self.lname(&tasty, tasty.source_name(n));
        let Some(TType::Applied(f, _)) = cv.local_val_types.get(&local) else { return false };
        let f = f.clone();
        let mut cx = super::MapCx::new(cv.file);
        let ctor = self.map_type_ctor(&mut cx, &f);
        let Type::Ctor(c) = self.types.get(ctor) else { return false };
        self.syms.class(c).tparams.iter().any(|&p| self.syms.tparam(p).name == name)
    }

    /// The upper bound of the one wildcard argument in the declared type of the local `addr`.
    fn wildcard_capture(&self, cv: &Conv, addr: Addr) -> Option<TType> {
        let TType::Applied(_, args) = cv.local_val_types.get(&addr)? else { return None };
        let mut bounds = args.iter().filter_map(|a| match a {
            TType::Bounds(_, hi) => Some((**hi).clone()),
            _ => None,
        });
        let hi = bounds.next()?;
        bounds.next().is_none().then_some(hi)
    }

    /// `x.T` for an `x` declared of an opaque type over `C[? <: U]` and the parameter `T` of
    /// `C`: scalac's capture of the wildcard, read as the wildcard, so that `x`, which is a
    /// `C[? <: U]` where the opaque type is transparent, passes for a `C[x.T]`. Only the opaque
    /// type's scope pickles it (`genericArrayOps[arr.T](arr)` for an `arr: IArray[T]`).
    fn opaque_wildcard_capture(&mut self, cv: &Conv, t: &TType) -> Option<TypeId> {
        let TType::TypeRef(prefix, n) = t else { return None };
        let TType::LocalTerm(addr, _) = &**prefix else { return None };
        let declared @ TType::Applied(..) = cv.local_val_types.get(addr)? else { return None };
        let declared = declared.clone();
        let mut cx = super::MapCx::new(cv.file);
        let declared = self.map_type(&mut cx, &declared);
        let Type::Class(o, _) = self.types.get(declared) else { return None };
        if self.syms.class(o).kind != ClassKind::Opaque {
            return None;
        }
        let under = self.opaque_erasure(declared)?;
        let Type::Class(c, cargs) = self.types.get(under) else { return None };
        let cargs = self.types.items(cargs).to_vec();
        let mut wilds = cargs.iter().enumerate().filter(|&(_, &a)| matches!(self.types.get(a), Type::BoundedWild(..)));
        let (index, &wild) = wilds.next()?;
        if wilds.next().is_some() {
            return None;
        }
        let name = self.interner.intern(&cv.tasty.name(cv.tasty.source_name(*n)));
        let param = *self.syms.class(c).tparams.get(index)?;
        (self.syms.tparam(param).name == name).then_some(wild)
    }

    /// `Array[?]#T`: a type parameter projected from a type with a wildcard argument.
    fn projects_wildcard(&self, t: &TType) -> bool {
        match t {
            TType::TypeRef(prefix, _) => matches!(&**prefix, TType::Applied(_, args) if args.iter().any(|a| matches!(a, TType::Bounds(..)))),
            TType::Applied(f, args) => self.projects_wildcard(f) || args.iter().any(|a| self.projects_wildcard(a)),
            TType::And(a, b) | TType::Or(a, b) => self.projects_wildcard(a) || self.projects_wildcard(b),
            TType::Annotated(inner, _) | TType::ByName(inner) => self.projects_wildcard(inner),
            _ => false,
        }
    }

    fn mentions_wildcard_capture(&self, cv: &Conv, t: &TType) -> bool {
        let captured = |prefix: &TType| matches!(prefix, TType::LocalTerm(addr, _) if self.wildcard_capture(cv, *addr).is_some());
        match t {
            TType::TypeRef(prefix, _) => captured(prefix),
            TType::LocalType(_, Some(prefix)) => captured(prefix),
            TType::Applied(f, args) => self.mentions_wildcard_capture(cv, f) || args.iter().any(|a| self.mentions_wildcard_capture(cv, a)),
            TType::And(a, b) | TType::Or(a, b) => self.mentions_wildcard_capture(cv, a) || self.mentions_wildcard_capture(cv, b),
            TType::Annotated(inner, _) | TType::ByName(inner) => self.mentions_wildcard_capture(cv, inner),
            _ => false,
        }
    }

    /// The member a super accessor `super$pkg$Trait$$m` stands for.
    fn super_accessor_member(&mut self, cv: &Conv, n: u32) -> Option<Name> {
        let text = cv.tasty.name(cv.tasty.source_name(n));
        let rest = text.strip_prefix("super$")?;
        let (_, member) = rest.rsplit_once("$$")?;
        Some(self.interner.intern(member))
    }

    fn mentions_local_type(&self, cv: &Conv, t: &TType) -> bool {
        match t {
            TType::This(_) => self.this_of_local_class(cv, t),
            // `{ type F[A] = .. }#F`, a type lambda spelled as a refinement's projection, over the
            // body's own type parameters.
            TType::TypeRef(prefix, _) if matches!(&**prefix, TType::Refined(..)) => self.mentions_local_type(cv, prefix),
            TType::TypeRef(prefix, _) => self.path_mentions_local(cv, prefix) || self.this_of_local_class(cv, prefix),
            TType::TermRef(prefix, _) if matches!(&**prefix, TType::This(_)) => true,
            TType::LocalTerm(..) | TType::TermRef(..) => self.path_mentions_local(cv, t),
            TType::LocalType(addr, prefix) => {
                cv.local_types.contains_key(addr)
                    || Decoder::new(&cv.tasty).tag_at(*addr) == tags::BIND
                    || prefix.as_deref().map_or(false, |p| self.path_mentions_local(cv, p) || self.this_of_local_class(cv, p))
            }
            TType::Applied(f, args) => self.mentions_local_type(cv, f) || args.iter().any(|a| self.mentions_local_type(cv, a)),
            TType::ParamRef(binder, _) => cv.poly_binders.iter().any(|(b, _)| b == binder),
            TType::Lambda { params, result, .. } => params.iter().any(|p| self.mentions_local_type(cv, &p.info)) || self.mentions_local_type(cv, result),
            TType::Refined(parent, members) => {
                self.mentions_local_type(cv, parent) || members.iter().any(|(_, info)| info.as_ref().map_or(false, |t| self.mentions_local_type(cv, t)))
            }
            TType::And(a, b) | TType::Or(a, b) => self.mentions_local_type(cv, a) || self.mentions_local_type(cv, b),
            TType::ByName(inner) | TType::Annotated(inner, _) | TType::Flexible(inner) | TType::Alias(inner) | TType::BoundedAlias(_, inner) => self.mentions_local_type(cv, inner),
            TType::Bounds(lo, hi) => self.mentions_local_type(cv, lo) || self.mentions_local_type(cv, hi),
            _ => false,
        }
    }

    /// The type of a parameter of a local method or class: a by-name or repeated marker is the
    /// AST's, over the type it wraps.
    fn conv_param_type(&mut self, cv: &mut Conv, t: &TType) -> TyExprId {
        let tasty = cv.tasty.clone();
        if let TType::ByName(inner) = t {
            let i = self.conv_type(cv, inner);
            return cv.ty(TyExpr::ByName(i));
        }
        if let Some(elem) = repeated_element(&tasty, t) {
            let e = self.conv_type(cv, elem);
            return cv.ty(TyExpr::Repeated(e));
        }
        self.conv_type(cv, t)
    }

    /// The type of a `new`: the class applied to its arguments as the AST spells it, so that the
    /// constructor takes them as explicit type arguments.
    /// `[A] =>> C[F, A]`, a class nested in a generic class seen through its prefix: the class
    /// and the outer arguments that precede the lambda's parameters.
    fn outer_applied_class(&self, ctor: TypeId) -> Option<(ClassId, Vec<TypeId>)> {
        let Type::Lambda(ps, body) = self.types.get(ctor) else { return None };
        let Type::Class(c, args) = self.types.get(body) else { return None };
        let (ps, args) = (self.types.items(ps), self.types.items(args));
        let n = args.len().checked_sub(ps.len()).filter(|&n| n > 0)?;
        (args[n..] == *ps).then(|| (c, args[..n].to_vec()))
    }

    fn conv_new_type(&mut self, cv: &mut Conv, t: &TType, targs: &[TType], inferred: bool) -> TyExprId {
        // `new C[T](args)`: `new` names the class and the constructor call applies it to the
        // type arguments, which are the type's arguments in the AST.
        if !targs.is_empty() && !matches!(t, TType::Applied(..)) {
            let mut outer_args = Vec::new();
            let head = if self.mentions_local_type(cv, t) {
                self.conv_type(cv, t)
            } else {
                let mut cx = super::MapCx::new(cv.file);
                let ctor = self.map_type_ctor(&mut cx, t);
                // A class nested in a generic class through its prefix, `[A] =>> Impl[F, A]`:
                // the class, given the outer arguments before the written ones.
                match self.outer_applied_class(ctor) {
                    Some((c, outer)) => {
                        outer_args = outer;
                        let ctor = self.types.mk(Type::Ctor(c));
                        cv.ty(TyExpr::Resolved(ctor))
                    }
                    None => cv.ty(TyExpr::Resolved(ctor)),
                }
            };
            if targs.iter().any(|a| self.mentions_wildcard_capture(cv, a)) {
                return head;
            }
            let mut items: Vec<TyExprId> = outer_args.into_iter().map(|t| cv.ty(TyExpr::Resolved(t))).collect();
            items.extend(targs.iter().map(|a| self.conv_type_arg(cv, a)));
            let l = push_list(&mut cv.ast.ty_lists, &items);
            if inferred {
                cv.ast.inferred_type_lists.push(l.start);
            }
            return cv.ty(TyExpr::Apply(head, l));
        }
        if let (TType::Applied(head, _), true) = (t, self.mentions_wildcard_capture(cv, t)) {
            let mut cx = super::MapCx::new(cv.file);
            let ctor = self.map_type_ctor(&mut cx, head);
            return cv.ty(TyExpr::Resolved(ctor));
        }
        if self.mentions_local_type(cv, t) {
            return self.conv_type(cv, t);
        }
        let mut cx = super::MapCx::new(cv.file);
        let resolved = self.map_type(&mut cx, t);
        if let Type::Class(c, args) = self.types.get(resolved) {
            let items: Vec<TypeId> = self.types.items(args).to_vec();
            if !items.is_empty() {
                let ctor = self.types.mk(Type::Ctor(c));
                let head = cv.ty(TyExpr::Resolved(ctor));
                let targs: Vec<TyExprId> = items.into_iter().map(|a| cv.ty(TyExpr::Resolved(a))).collect();
                let l = push_list(&mut cv.ast.ty_lists, &targs);
                return cv.ty(TyExpr::Apply(head, l));
            }
        }
        cv.ty(TyExpr::Resolved(resolved))
    }

    // ---- paths ----

    /// A member of a scala-library object that the lean std keeps in another object: the
    /// numeric instances of `Numeric` live in the companions of `BigDecimal` and `BigInt`,
    /// whose file a program's own `scala.math` may take the place of.
    fn std_member_home(&mut self, cv: &mut Conv, prefix: &TType, name: Name) -> Option<ExprId> {
        const HOMES: &[(&str, &str, &str)] = &[
            ("Numeric", "BigDecimalIsFractional", "BigDecimal"),
            ("Numeric", "BigIntIsIntegral", "BigInt"),
        ];
        if !self.std_binds() {
            return None;
        }
        let TType::TermRef(pkg, obj) = prefix else { return None };
        let TType::Package(p) = &**pkg else { return None };
        if cv.tasty.name(*p) != "scala.math" {
            return None;
        }
        let obj = cv.tasty.simple(cv.tasty.source_name(*obj))?;
        let member = self.name_str(name).to_string();
        let home = HOMES.iter().find(|(o, m, _)| *o == obj && *m == member)?.2;
        let mut e = cv.expr(Expr::Ident(names::ROOT));
        for seg in ["scala", "math", home] {
            let n = self.interner.intern(seg);
            e = cv.expr(Expr::Select(e, n));
        }
        Some(cv.expr(Expr::Select(e, name)))
    }

    /// A block's import as the source form, resolved by name when the body is typed: an
    /// absolute path from `_root_`, with a package object's members at the package's level as
    /// the loader spills them.
    fn conv_import(&mut self, cv: &mut Conv, path: &TType, selectors: &[(u32, Option<u32>)], in_block: bool) -> Option<Stmt> {
        // The body names its members by symbol: an import matters for the givens it brings,
        // which a package or an object holds, or, in a block, a local value's member named by
        // the import (`import init.{Underlying as Init}`); a value's wildcard (`import ops.*`
        // for a parameter `ops`) is left out.
        let of_value = !self.is_static_import_path(cv, path);
        if of_value && !(in_block && matches!(path, TType::LocalTerm(..))) {
            return None;
        }
        let path = self.import_path_names(cv, path)?;
        let start = cv.ast.local_imports.len() as u32;
        for &(from, to) in selectors {
            let sel = match cv.tasty.simple(from) {
                Some("") | Some("_") if of_value => continue,
                Some("") => ImportSel::Given,
                Some("_") => ImportSel::Wildcard,
                _ => ImportSel::Name(self.conv_name(cv, from), to.map(|n| self.conv_name(cv, n))),
            };
            cv.ast.local_imports.push(ast::Import { path: path.clone(), sel, span: cv.span, selector_span: cv.span, bound: None });
        }
        let len = cv.ast.local_imports.len() as u32 - start;
        if len == 0 {
            return None;
        }
        cv.ast.import_stmts.push(ListRef { start, len });
        Some(Stmt::Import(cv.ast.import_stmts.len() as u32 - 1))
    }

    fn is_static_import_path(&mut self, cv: &mut Conv, t: &TType) -> bool {
        match t {
            TType::Package(_) => true,
            TType::TermRef(..) => self.static_path_scope(cv, t).is_some(),
            TType::LocalTerm(addr, _) => {
                !cv.locals.contains_key(addr) && self.term_sym(cv, *addr).map_or(false, |s| matches!(self.syms.sym(s).kind, SymKind::Object(_)))
            }
            TType::This(inner) => self.this_class_of(cv, inner).map_or(false, |c| self.syms.class(c).kind == ClassKind::Object),
            _ => false,
        }
    }

    fn import_path_names(&mut self, cv: &mut Conv, t: &TType) -> Option<Vec<Name>> {
        match t {
            TType::Package(p) => {
                let text = cv.tasty.name(*p);
                let mut names = vec![names::ROOT];
                for seg in text.split('.').filter(|s| !s.is_empty() && !matches!(*s, "_root_" | "<root>" | "<empty>")) {
                    names.push(self.interner.intern(seg));
                }
                Some(names)
            }
            TType::TermRef(prefix, n) => {
                let name = self.conv_name(cv, *n);
                let spilled = self.is_package_path(cv, prefix) && super::types::is_package_object_name(cv.tasty.simple(cv.tasty.source_name(*n)));
                // A member of an enclosing class's `this` is in scope by its name.
                if let TType::This(inner) = &**prefix {
                    if let Some(c) = self.this_class_of(cv, inner) {
                        if self.syms.class(c).kind != ClassKind::Object && cv.this_chain.contains(&c) {
                            return Some(vec![name]);
                        }
                    }
                }
                let mut names = self.import_path_names(cv, prefix)?;
                if !spilled {
                    names.push(name);
                }
                Some(names)
            }
            TType::LocalTerm(addr, _) => {
                if let Some(name) = cv.locals.get(addr) {
                    return Some(vec![*name]);
                }
                let s = self.term_sym(cv, *addr)?;
                let (owner, name) = (self.syms.sym(s).owner, self.syms.sym(s).name);
                if let Owner::Class(k) = owner {
                    if self.syms.class(k).kind != ClassKind::Object {
                        return cv.this_chain.contains(&k).then(|| vec![name]);
                    }
                }
                let mut names = self.owner_path_names(owner)?;
                names.push(name);
                Some(names)
            }
            TType::This(inner) => {
                let c = self.this_class_of(cv, inner)?;
                if self.syms.class(c).kind != ClassKind::Object {
                    return None;
                }
                let mut names = self.owner_path_names(self.syms.class(c).owner)?;
                names.push(self.syms.class(c).name);
                Some(names)
            }
            _ => None,
        }
    }

    /// The segments from `_root_` that name the members of `owner`; a package object's
    /// members stand at its package's level.
    fn owner_path_names(&self, owner: Owner) -> Option<Vec<Name>> {
        match owner {
            Owner::Package(p) => {
                let mut names = Vec::new();
                let mut at = p;
                while at != ROOT_PKG {
                    let info = self.syms.pkg(at);
                    names.push(info.name);
                    at = info.parent?;
                }
                names.push(names::ROOT);
                names.reverse();
                Some(names)
            }
            Owner::Class(c) => {
                let info = self.syms.class(c);
                let mut names = self.owner_path_names(info.owner)?;
                if !super::types::is_package_object_name(Some(&self.name_str(info.name))) {
                    names.push(info.name);
                }
                Some(names)
            }
            Owner::Local => None,
        }
    }

    /// The package before a package object selected as a term (`scala.math` of
    /// `scala.math.package`), converted, when the term `q` names one.
    fn package_object_owner(&mut self, cv: &mut Conv, q: &Term) -> Option<ExprId> {
        let package_object = |cv: &Conv, m: u32| super::types::is_package_object_name(cv.tasty.simple(cv.tasty.source_name(m)));
        match &q.kind {
            TermKind::Ident(m, TType::TermRef(prefix, _)) if package_object(cv, *m) && self.is_package_path(cv, prefix) => {
                Some(self.conv_path(cv, prefix))
            }
            TermKind::Select(owner, m) | TermKind::SelectIn(owner, m, ..) if package_object(cv, *m) && self.is_package_term(cv, owner) => {
                Some(self.conv_expr(cv, owner))
            }
            _ => None,
        }
    }

    /// Whether the term names a package: a package path, or identifiers and selections that
    /// walk the package tree from the root or from a package the innermost term refers to
    /// (`scala.concurrent` of `scala.concurrent.package`, with `scala` a package path).
    fn is_package_term(&mut self, cv: &Conv, t: &Term) -> bool {
        let mut names = Vec::new();
        let mut at = t;
        let mut pkg = ROOT_PKG;
        loop {
            match &at.kind {
                TermKind::Path(ty) => {
                    match self.package_of_path(cv, ty) {
                        Some(p) => pkg = p,
                        None => return false,
                    }
                    break;
                }
                TermKind::Ident(n, ty) => {
                    match self.package_of_path(cv, ty) {
                        Some(p) => pkg = p,
                        None => names.push(self.conv_name(cv, *n)),
                    }
                    break;
                }
                TermKind::Select(inner, n) | TermKind::SelectIn(inner, n, ..) => {
                    names.push(self.conv_name(cv, *n));
                    at = inner;
                }
                _ => return false,
            }
        }
        for &name in names.iter().rev() {
            match self.syms.pkg(pkg).entries.get(&name).and_then(|e| e.pkg) {
                Some(p) => pkg = p,
                None => return false,
            }
        }
        true
    }

    /// Whether `t` names a package: a package reference, or a package selected from its parent
    /// as a term (`scala.math` before its package object's `min`).
    fn is_package_path(&mut self, cv: &Conv, t: &TType) -> bool {
        matches!(t, TType::Package(_)) || self.package_of_path(cv, t).is_some()
    }

    /// The package `t` refers to, when it names one.
    fn package_of_path(&mut self, cv: &Conv, t: &TType) -> Option<PkgId> {
        fn walk(t: &mut Worker, cv: &Conv, ty: &TType) -> Option<PkgId> {
            match ty {
                TType::Package(p) => {
                    let text = cv.tasty.name(*p);
                    let mut pkg = ROOT_PKG;
                    for seg in text.split('.').filter(|s| !s.is_empty() && !matches!(*s, "_root_" | "<root>" | "<empty>")) {
                        let n = t.interner.lookup(seg)?;
                        pkg = t.syms.pkg(pkg).entries.get(&n).and_then(|e| e.pkg)?;
                    }
                    Some(pkg)
                }
                TType::This(inner) => walk(t, cv, inner),
                TType::TermRef(prefix, n) => {
                    let parent = walk(t, cv, prefix)?;
                    let name = t.conv_name(cv, *n);
                    t.syms.pkg(parent).entries.get(&name).and_then(|e| e.pkg)
                }
                _ => None,
            }
        }
        walk(self, cv, t)
    }

    /// The class a product's SAM conversion of the lambda at `anonfun` made, as its kind 2
    /// record has it: the class's name in its producer's build, and the source (among
    /// `Loaded::product_sources`) and offset of the lambda.
    fn product_sam_class(&mut self, cv: &Conv, anonfun: Addr) -> Option<(String, u32, u32)> {
        use crate::tasty::origins::{self, ClassOrigin, Found};
        let loaded = self.loaded.as_ref()?;
        let Found::Read(o) = &loaded.file(cv.file).provenance else { return None };
        let i = o.classes.binary_search_by_key(&anonfun, |&(a, _)| a).ok()?;
        let origin = &o.classes[i].1;
        let ClassOrigin::Anonymous { at, .. } = origin else { return None };
        let name = origins::class_name(o, origin, "")?;
        let (key, token) = match at.0 {
            0 => (o.key.clone(), o.token),
            r => o.sources.get(r as usize - 1)?.clone(),
        };
        let offset = at.1;
        let source = self.loaded_mut().product_source_index(&key, token);
        Some((name, source, offset))
    }

    /// The expansions of a product's file to replay (kind 8 version 2), in a
    /// build that records expansions (one of JavaScript), each with its method found.
    fn prepare_replay(&mut self, cv: &mut Conv) {
        if !self.records_expansions() || self.writes_products || !self.converts_product(cv) {
            return;
        }
        let callees: Vec<(Addr, crate::tasty::origins::Callee)> = match &self.loaded.as_ref().unwrap().file(cv.file).provenance {
            crate::tasty::origins::Found::Read(o) if !o.callees.is_empty() => o.callees.clone(),
            _ => return,
        };
        for (at, c) in callees {
            let Some(m) = self.product_callee(&c) else {
                // Its expansion reads as no expansion of a method: the body fails where it is
                // typed, naming the callee, rather than replaying none of its records.
                cv.replay_unresolved.insert(at, format!("{}.{}{}", c.owner, c.name, if c.signature.is_empty() { String::new() } else { format!(" of signature {}", c.signature) }));
                continue;
            };
            self.loaded_mut().note_callee_source(m, cv.file, c.at);
            cv.replay_callees.insert(at, m);
            cv.replay_leaves.extend(c.leaves.iter().map(|&a| (a, ())));
            cv.replay_tests.extend(c.leaf_tests.iter().map(|&a| (a, ())));
        }
    }

    /// The method a product's `INLINED` expands, from its owner's qualified name, its name and
    /// its signature: a top-level class's or object's member, or a top-level definition of a
    /// `<file>$package`, the one alternative of its name whose parameters erase as the
    /// signature states. `None` for another owner or where no one alternative does.
    fn product_callee(&mut self, c: &crate::tasty::origins::Callee) -> Option<SymId> {
        let segs: Vec<&str> = c.owner.split('.').filter(|s| !s.is_empty()).collect();
        let (&top, rest) = segs.split_last()?;
        // The packages, then the top-level class or object, then the classes and objects nested
        // in it (`nest.A$.Inner$`).
        let mut p = crate::symbols::ROOT_PKG;
        let mut i = 0;
        while i < rest.len() && !rest[i].ends_with('$') {
            let n = self.interner.intern(rest[i]);
            match self.demand_pkg(p, n) {
                Some(q) => p = q,
                None => break,
            }
            i += 1;
        }
        self.enter_pkg_objects(p);
        let path: Vec<&str> = rest[i..].iter().copied().chain(std::iter::once(top)).collect();
        let n = self.interner.intern(path[0].trim_end_matches('$'));
        self.load_pkg_member(p, n);
        let entry = self.syms.pkg(p).entries.get(&n)?;
        let mut owner = match path[0].ends_with('$') {
            true => match self.syms.sym(entry.term?).kind {
                SymKind::Object(k) => k,
                _ => return None,
            },
            false => entry.class?,
        };
        for seg in &path[1..] {
            self.complete_class(owner);
            let n = self.interner.intern(seg.trim_end_matches('$'));
            owner = match seg.ends_with('$') {
                true => match self.syms.sym(*self.syms.class(owner).members.get(&n)?).kind {
                    SymKind::Object(k) => k,
                    _ => return None,
                },
                false => *self.syms.class(owner).nested.get(&n)?,
            };
        }
        let name = self.interner.intern(&c.name);
        self.complete_class(owner);
        let loaded = self.loaded.as_ref().unwrap();
        let found: Vec<SymId> = if loaded.product_packages.contains_key(&owner) {
            loaded.product_package_members.iter().filter(|&(&s, &o)| o == owner && self.syms.sym(s).name == name).map(|(&s, _)| s).collect()
        } else {
            self.syms.class(owner).members.get(&name).copied().into_iter().collect()
        };
        let alternatives: Vec<SymId> = found.iter().flat_map(|&s| self.syms.alternatives(s).map_or(vec![s], |a| a.to_vec())).collect();
        match alternatives.as_slice() {
            [s] => Some(*s),
            _ => match self.alternatives_of_signature(&alternatives, &c.signature).as_slice() {
                [s] if !c.signature.is_empty() => Some(*s),
                _ => None,
            },
        }
    }

    /// Whether a product's tree is a `()` the writer made where the source wrote nothing: its
    /// position is no span of the source.
    fn synthetic_unit(&mut self, cv: &Conv, t: &Term) -> bool {
        if !matches!(t.kind, TermKind::Const(crate::tasty::tree::Const::Unit)) {
            return false;
        }
        let positions = self.file_positions(cv.file);
        let Some(section) = positions.section.as_ref() else { return false };
        section.entry(t.at).map_or(true, |p| p.point.is_none() && p.start == p.end)
    }

    /// The argument of a product's `summon[T]`, which teq's writer writes as scalac's expansion
    /// of the transparent `Predef.summon`: an `INLINED` from the class `Predef$` whose expansion
    /// is its parameter's proxy, an `INLINED` without a call.
    fn product_summoned<'t>(&self, cv: &Conv, expansion: &'t Term, call: Option<&Term>, bindings: &[Stat]) -> Option<&'t Term> {
        if !cv.product || !bindings.is_empty() {
            return None;
        }
        let Some(TermKind::Path(TType::TypeRef(prefix, n))) = call.map(|c| &c.kind) else { return None };
        let TType::Package(p) = &**prefix else { return None };
        let predef = matches!(cv.tasty.names.get(*n as usize), Some(TName::ObjectClass(_))) && cv.tasty.simple(cv.tasty.source_name(*n)) == Some("Predef");
        if cv.tasty.simple(*p) != Some("scala") || !predef {
            return None;
        }
        match &expansion.kind {
            TermKind::Inlined { expansion: arg, call: None, bindings, .. } if bindings.is_empty() => Some(arg),
            _ => None,
        }
    }

    /// Whether a product's `q.m()` is teq's writer's application of a Java method the source
    /// selected without `()` (`s.toUpperCase`), the selection spanning the call: read
    /// unapplied, the typer picks the std's member the source's did.
    fn product_auto_applied(&mut self, cv: &Conv, apply: &Term, f: &Term) -> bool {
        if !cv.product || !matches!(f.kind, TermKind::Select(..) | TermKind::SelectIn(..)) {
            return false;
        }
        let positions = self.file_positions(cv.file);
        let Some(section) = positions.section.as_ref() else { return false };
        matches!((section.entry(apply.at), section.entry(f.at)), (Some(a), Some(s)) if a.start == s.start && a.end == s.end)
    }

    /// Whether a product's call selects a library member whose `Char` argument teq's writer
    /// widened to its `Int` parameter (the inverse table's `widened` form, `indexOf(scala.Int)`).
    fn product_widened(&self, cv: &Conv, f: &Term) -> bool {
        if !cv.product {
            return false;
        }
        let mut head = f;
        while let TermKind::TypeApply(g, ..) = &head.kind {
            head = g;
        }
        let TermKind::SelectIn(_, n, owner, _) = &head.kind else { return false };
        let Some(crate::tasty::TName::Signed { original, result, params, .. }) = cv.tasty.names.get(*n as usize) else { return false };
        let name = cv.tasty.name(*original);
        let params: Vec<String> = params.iter().map(|&p| if p < 0 { format!("[{}]", -p) } else { cv.tasty.name(p as u32) }).collect();
        let shape = format!("{}:{}", params.join(","), cv.tasty.name(*result));
        let owner = self.qualified_type_name(cv, owner);
        crate::tasty::write::widened_library_members().any(|(o, m, s)| Some(o) == owner.as_deref() && m == name && s == shape)
    }

    /// The qualified name of a class a type names by its package (`java.lang.String`).
    fn qualified_type_name(&self, cv: &Conv, t: &TType) -> Option<String> {
        match t {
            TType::TypeRef(prefix, n) => match &**prefix {
                TType::Package(p) => Some(format!("{}.{}", cv.tasty.name(*p), cv.tasty.name(cv.tasty.source_name(*n)))),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether `head` selects a written overload of a var's setter name of a product's local
    /// class by its signature, which is no assignment of the var.
    fn calls_written_setter(&self, cv: &Conv, head: &Term) -> bool {
        let TermKind::SelectIn(_, n, TType::LocalType(class, _), _) = &head.kind else { return false };
        let Some(TName::Signed { params, .. }) = cv.tasty.names.get(*n as usize) else { return false };
        let [p] = params.as_slice() else { return false };
        let Ok(p) = NameRef::try_from(*p) else { return false };
        let Some(name) = cv.tasty.simple(cv.tasty.source_name(*n)) else { return false };
        cv.written_setters.contains_key(&(*class, name.to_string(), cv.tasty.name(p)))
    }

    /// `x` of the widened argument `scala.Char.char2int(x)`.
    fn char_widened_arg<'t>(&self, cv: &Conv, a: &'t Term) -> Option<&'t Term> {
        let TermKind::Apply(g, args) = &a.kind else { return None };
        let [x] = args.as_slice() else { return None };
        let (q, n) = match &g.kind {
            TermKind::SelectIn(q, n, ..) | TermKind::Select(q, n) => (q, *n),
            _ => return None,
        };
        let char_object = match &q.kind {
            TermKind::Path(TType::TermRef(prefix, o)) => matches!(&**prefix, TType::Package(p) if cv.tasty.name(*p) == "scala") && cv.tasty.simple(cv.tasty.source_name(*o)) == Some("Char"),
            _ => false,
        };
        (char_object && cv.tasty.simple(cv.tasty.source_name(n)) == Some("char2int")).then_some(x)
    }

    /// Whether the class being converted is a product directory's, whose rules differ from a
    /// jar's.
    fn converts_product(&self, cv: &Conv) -> bool {
        cv.product
    }

    /// The member a product's inline accessor `inline$<name>` (`inline$<prefix>$$<name>` in a
    /// class another can extend) reads, which teq's writer gives a class for its inline bodies'
    /// reads of private members (`typer::accessors`); a setter's `<name>_=` keeps its suffix.
    fn product_accessor_target(&mut self, cv: &Conv, n: u32) -> Option<Name> {
        if !self.converts_product(cv) {
            return None;
        }
        if !cv.tasty.is_inline_accessor(n) {
            return None;
        }
        let name = cv.tasty.name(cv.tasty.source_name(n));
        let rest = name.strip_prefix("inline$")?;
        let target = rest.rsplit_once("$$").map_or(rest, |(_, t)| t);
        Some(self.interner.intern(target))
    }

    /// What an accessor's member is read on: its qualifier, or the companion object where the
    /// qualifier's class (`C.this`) has no member of the name and its companion has (the
    /// accessor `inline$p$C$$hidden` of `object C`'s private `hidden`).
    fn accessor_receiver(&mut self, cv: &mut Conv, q: &Term, member: Name) -> ExprId {
        let this = match &q.kind {
            TermKind::QualThis(t) => Some(t),
            TermKind::Path(TType::This(t)) => Some(&**t),
            _ => None,
        };
        if let Some(t) = this {
            if let Some(k) = self.this_class_of(cv, t) {
                self.complete_class(k);
                let companion = self.syms.class(k).companion.filter(|&o| self.syms.class(o).kind == ClassKind::Object);
                if let Some(o) = companion {
                    self.complete_class(o);
                }
                if let Some(o) = companion.filter(|&o| !self.syms.class(k).members.contains_key(&member) && self.syms.class(o).members.contains_key(&member)) {
                    if let Some(m) = self.syms.class(o).module_sym {
                        return cv.expr(Expr::SymRef(m));
                    }
                }
                // `C.this` as `C`, where the accessor's member is the class's own: an expansion
                // over a subclass's instance reads `C`'s private member, which the accessor's
                // body names and the subclass has not.
                if self.syms.class(k).members.contains_key(&member) {
                    let qe = self.conv_expr(cv, q);
                    let ty = self.syms.this_type(k);
                    let ty = cv.ty(TyExpr::Resolved(ty));
                    return cv.expr(Expr::Typed(qe, ty));
                }
            }
        }
        self.conv_expr(cv, q)
    }

    /// `r.inline$..$x_=(v)` of a product, an inline accessor's setter: the assignment
    /// `r.x = v` it stands for.
    fn conv_accessor_assign(&mut self, cv: &mut Conv, f: &Term, args: &[Term]) -> Option<ExprId> {
        let (TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..)) = &f.kind else { return None };
        let [v] = args else { return None };
        let target = self.product_accessor_target(cv, *n)?;
        let var = self.name_str(target).strip_suffix("_=")?.to_string();
        let var = self.interner.intern(&var);
        let qe = self.accessor_receiver(cv, q, var);
        let lhs = cv.expr(Expr::Select(qe, var));
        let rhs = self.conv_expr(cv, v);
        Some(cv.expr(Expr::Assign(lhs, rhs)))
    }

    /// Whether `prefix.n` is the module of a product's `<file>$package` object, which this build
    /// reads as its package's top-level definitions.
    fn names_product_package(&mut self, cv: &Conv, prefix: &TType, n: NameRef) -> bool {
        let product = self.converts_product(cv);
        let name = cv.tasty.name(cv.tasty.source_name(n));
        if !product || !name.ends_with("$package") {
            return false;
        }
        let Some(p) = self.package_of_path(cv, prefix) else { return false };
        let n = self.interner.intern(&name);
        // The object enters with its package's files, which this reference may be the first to
        // open.
        self.enter_pkg_objects(p);
        let object = self.syms.pkg(p).entries.get(&n).and_then(|e| e.term).and_then(|s| match self.syms.sym(s).kind {
            SymKind::Object(c) => Some(c),
            _ => None,
        });
        object.map_or(false, |c| self.loaded.as_ref().unwrap().product_packages.contains_key(&c))
    }

    fn names_absent_package_object(&mut self, cv: &Conv, prefix: &TType, n: NameRef) -> bool {
        if !super::types::is_package_object_name(cv.tasty.simple(cv.tasty.source_name(n))) {
            return false;
        }
        let Some(p) = self.package_of_path(cv, prefix) else { return false };
        let n = self.interner.intern(&cv.tasty.name(cv.tasty.source_name(n)));
        self.enter_pkg_objects(p);
        self.load_pkg_member(p, n);
        self.syms.pkg(p).entries.get(&n).and_then(|e| e.term).is_none()
    }

    fn conv_path(&mut self, cv: &mut Conv, t: &TType) -> ExprId {
        match t {
            TType::Package(p) => {
                let text = cv.tasty.name(*p);
                let mut e = cv.expr(Expr::Ident(names::ROOT));
                for seg in text.split('.').filter(|s| !s.is_empty() && !matches!(*s, "_root_" | "<root>" | "<empty>")) {
                    let n = self.interner.intern(seg);
                    e = cv.expr(Expr::Select(e, n));
                }
                e
            }
            TType::TermRef(..) | TType::LocalTerm(..) if self.names_accessor(cv, t) => {
                let term = Term::new(cv.at, TermKind::Path(t.clone()));
                match self.accessor_of(cv, &term).and_then(|acc| self.conv_accessor(cv, acc)) {
                    Some(e) => e,
                    None => cv.refuse("an inline accessor of that shape"),
                }
            }
            // The object of a product's top-level definitions is their package: `p.F$package`
            // is `p`.
            TType::TermRef(prefix, n) if matches!(&**prefix, TType::Package(_)) && self.names_product_package(cv, prefix, *n) => self.conv_path(cv, prefix),
            // The std defines a package object's members at its package's level, with no object:
            // `scala.compiletime.package$package` is `scala.compiletime`.
            TType::TermRef(prefix, n) if matches!(&**prefix, TType::Package(_)) && self.names_absent_package_object(cv, prefix, *n) => self.conv_path(cv, prefix),
            TType::TermRef(prefix, n) => {
                let name = self.conv_name(cv, *n);
                // `p.package.x` is the member `x` of the package object of `p`, which the
                // loader spills into the package and the std defines at the package's level.
                // `scala.Predef.x` without a jar that defines `Predef` is the std's `x` too.
                if let TType::TermRef(pkg, obj) = &**prefix {
                    if super::types::is_package_object_name(cv.tasty.simple(cv.tasty.source_name(*obj))) && self.is_package_path(cv, pkg) {
                        let q = self.conv_path(cv, pkg);
                        return cv.expr(Expr::Select(q, name));
                    }
                    // `Predef.classOf[T]` is the compiler's intrinsic (dotty's `Predef_classOf`, which
                    // the typer folds to the class), typed from the bare name whichever `Predef`
                    // is loaded: scala-library's own is a stub that answers `null`.
                    if name == names::CLASS_OF && self.is_predef_path(cv, prefix) {
                        return cv.expr(Expr::Ident(name));
                    }
                    if self.is_unloaded_predef(cv, prefix) {
                        let q = self.conv_path(cv, pkg);
                        let name = if self.name_str(name) == "$conforms" { self.interner.intern("conforms") } else { name };
                        return cv.expr(Expr::Select(q, name));
                    }
                }
                if let Some(e) = self.std_member_home(cv, prefix, name) {
                    return e;
                }
                // A product's local class's `C.this.x` in its parents' calls: the parameter `x`.
                if let (TType::This(inner), Some((at, params))) = (&**prefix, cv.parent_params.as_ref()) {
                    if matches!(&**inner, TType::LocalType(a, _) if a == at) && params.contains(&name) {
                        return cv.expr(Expr::Ident(name));
                    }
                }
                // `O.this.m` for a member of an enclosing object names the member itself.
                if let TType::This(inner) = &**prefix {
                    if let Some(c) = self.this_class_of(cv, inner) {
                        if self.syms.class(c).kind == ClassKind::Object {
                            if let Some(s) = self.module_term(c, name).and_then(|r| r.sym()) {
                                return cv.expr(Expr::SymRef(s));
                            }
                        }
                    }
                }
                let q = self.conv_path(cv, prefix);
                cv.expr(Expr::Select(q, name))
            }
            TType::LocalTerm(addr, prefix) => {
                if let Some(name) = cv.locals.get(addr).copied() {
                    return cv.expr(Expr::Ident(name));
                }
                if cv.quotes_params.contains_key(addr) {
                    return self.summon_quotes(cv);
                }
                if let Some(var) = cv.type_givens.get(addr).copied() {
                    return self.summon_type_of(cv, var);
                }
                match self.term_sym_completing(cv, *addr) {
                    Some(s) => match self.syms.sym(s).owner {
                        // An inherited member is selected on the `this` that holds it, whose
                        // type arguments its type is seen through.
                        Owner::Class(k) if self.syms.class(k).kind != ClassKind::Object && !cv.this_chain.contains(&k) => {
                            let holder = cv.this_chain.iter().rev().position(|&c| c != LOCAL_CLASS && self.derives_from(c, k));
                            let this = match holder {
                                Some(0) => cv.expr(Expr::This),
                                Some(i) => {
                                    let c = cv.this_chain[cv.this_chain.len() - 1 - i];
                                    let ty = self.syms.class(c).base_types.first().map(|&(_, t)| t).unwrap_or(ERROR);
                                    let t = cv.ty(TyExpr::Resolved(ty));
                                    cv.expr(Expr::ThisOf(t))
                                }
                                // Inherited by an object the path names (`Companion.given` of a
                                // trait `Companion` extends): selected on that object.
                                None => match prefix {
                                    Some(p) => {
                                        let q = self.conv_path(cv, p);
                                        let name = self.syms.sym(s).name;
                                        return cv.expr(Expr::Select(q, name));
                                    }
                                    None => return cv.expr(Expr::SymRef(s)),
                                },
                            };
                            let name = self.syms.sym(s).name;
                            cv.expr(Expr::Select(this, name))
                        }
                        _ => cv.expr(Expr::SymRef(s)),
                    },
                    None => {
                        let tasty = cv.tasty.clone();
                        let name = Decoder::new(&tasty).name_at(*addr).map(|n| tasty.name(n)).unwrap_or_default();
                        // A companion's constructor `apply`, which the compiler synthesizes,
                        // named from inside the companion.
                        let n = self.interner.intern(&name);
                        let holder = cv.this_chain.iter().rev().copied().find(|&c| c != LOCAL_CLASS && self.syms.class(c).kind == ClassKind::Object);
                        match holder.filter(|&c| self.companion_class(c).is_some()) {
                            Some(_) if n == names::APPLY => {
                                let this = cv.expr(Expr::This);
                                cv.expr(Expr::Select(this, n))
                            }
                            _ => cv.refuse(format!("a reference to {} (the definition at {})", name, addr)),
                        }
                    }
                }
            }
            TType::This(inner) => self.conv_this(cv, inner),
            TType::Const(c) => self.conv_const(cv, c),
            other => cv.refuse(format!("a path of the shape {:?}", std::mem::discriminant(other))),
        }
    }

    /// The class that the type of a `C.this` names.
    fn this_class_of(&mut self, cv: &mut Conv, class: &TType) -> Option<ClassId> {
        let mut cx = super::MapCx::new(cv.file);
        match class {
            TType::LocalType(addr, _) => self.file_tables(cv.file).classes.get(addr).copied(),
            other => {
                let t = self.map_type(&mut cx, other);
                match self.types.get(t) {
                    Type::Class(k, _) => Some(k),
                    _ => None,
                }
            }
        }
    }

    /// `C.this`: `this` for the innermost class under conversion, the outer `this` of an
    /// enclosing one, and the object itself for a module.
    fn conv_this(&mut self, cv: &mut Conv, class: &TType) -> ExprId {
        let c = self.this_class_of(cv, class);
        // `$anon.this` of an enclosing local class from inside another: a val the enclosing
        // block binds to that `this` (`conv_block`).
        if let TType::LocalType(addr, _) = class {
            if cv.local_classes.contains(addr) && cv.local_classes.last() != Some(addr) {
                let name = match cv.self_vals.iter().find(|&&(a, _)| a == *addr) {
                    Some(&(_, n)) => n,
                    None => {
                        let n = self.interner.intern(&format!("$self{}", addr));
                        cv.self_vals.push((*addr, n));
                        n
                    }
                };
                return cv.expr(Expr::Ident(name));
            }
        }
        // The object of a product's top-level definitions is their package.
        if let Some(k) = c.filter(|k| self.loaded.as_ref().map_or(false, |l| l.product_packages.contains_key(k))) {
            if let Some(names) = self.owner_path_names(self.syms.class(k).owner) {
                let mut e = cv.expr(Expr::Ident(names[0]));
                for &n in &names[1..] {
                    e = cv.expr(Expr::Select(e, n));
                }
                return e;
            }
        }
        match c {
            Some(k) if cv.this_chain.last() == Some(&k) => cv.expr(Expr::This),
            // A local class is the innermost by construction.
            None if matches!(class, TType::LocalType(..)) => cv.expr(Expr::This),
            Some(k) if self.syms.class(k).kind == ClassKind::Object && !cv.this_chain.contains(&k) => {
                let ty = self.types.class(k, &[]);
                let t = cv.ty(TyExpr::Resolved(ty));
                cv.expr(Expr::ThisOf(t))
            }
            Some(k) => {
                let ty = self.syms.class(k).base_types.first().map(|&(_, t)| t).unwrap_or(ERROR);
                let t = cv.ty(TyExpr::Resolved(ty));
                cv.expr(Expr::ThisOf(t))
            }
            None => cv.expr(Expr::This),
        }
    }

    fn conv_const(&mut self, cv: &mut Conv, c: &Const) -> ExprId {
        let e = match c {
            Const::Unit => Expr::UnitLit,
            Const::Bool(b) => Expr::BoolLit(*b),
            Const::Int(v) => Expr::IntLit(*v as i64),
            // A byte or short constant keeps its type, as an explicit `Array.apply[Short](...)` needs.
            Const::Byte(v) => {
                let lit = cv.expr(Expr::IntLit(*v as i64));
                let ty = cv.ty(TyExpr::Resolved(self.b.t_byte));
                Expr::Typed(lit, ty)
            }
            Const::Short(v) => {
                let lit = cv.expr(Expr::IntLit(*v as i64));
                let ty = cv.ty(TyExpr::Resolved(self.b.t_short));
                Expr::Typed(lit, ty)
            }
            Const::Char(v) => Expr::CharLit(*v),
            Const::Long(v) => Expr::LongLit(*v),
            Const::Float(bits) => Expr::FloatLit(f32::from_bits(*bits)),
            Const::Double(bits) => Expr::DoubleLit(f64::from_bits(*bits)),
            Const::Str(n) => {
                let text = cv.tasty.name(*n);
                let s = cv.ast.add_str(text);
                Expr::StringLit(s)
            }
            Const::Null => Expr::NullLit,
            Const::Class(t) => {
                let ty = self.conv_type(cv, t);
                Expr::ClassOf(ty)
            }
        };
        cv.expr(e)
    }

    // ---- terms ----

    fn conv_expr(&mut self, cv: &mut Conv, t: &Term) -> ExprId {
        let e = cv.at(t.at, |cv| self.conv_expr_now(cv, t));
        // A node made of a part of the tree (`a + b`'s selection, `x.length()`'s, an anonymous
        // class's instance for its block) stands for the tree whole; an `INLINED`'s expansion
        // keeps its own.
        if cv.expr_at.get(e.idx()).map_or(false, |&a| a != t.at) && !matches!(t.kind, TermKind::Inlined { .. }) {
            cv.whole(e, t.at);
        }
        if cv.product && (!cv.replay_leaves.is_empty() || !cv.replay_tests.is_empty()) {
            if cv.replay_leaves.contains_key(&t.at) {
                cv.reader().replay.leaves.insert(e, ());
            }
            if cv.replay_tests.contains_key(&t.at) {
                cv.reader().replay.test_exprs.insert(e, ());
            }
        }
        e
    }

    fn conv_expr_now(&mut self, cv: &mut Conv, t: &Term) -> ExprId {
        match &t.kind {
            TermKind::Path(p) => self.conv_path(cv, p),
            TermKind::Const(c) => self.conv_const(cv, c),
            // `var x: T = _`, the zero of the variable's type.
            TermKind::Ident(n, _) if cv.tasty.simple(*n) == Some("_") => {
                let mut e = cv.expr(Expr::Ident(names::ROOT));
                for seg in ["scala", "compiletime", "uninitialized"] {
                    let n = self.interner.intern(seg);
                    e = cv.expr(Expr::Select(e, n));
                }
                e
            }
            TermKind::Ident(n, _) => {
                let name = self.conv_name(cv, *n);
                cv.expr(Expr::Ident(name))
            }
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) => {
                // A product's inline accessor (`r.inline$p$C$$m`): the member
                // it reads, as the source has it.
                if let Some(target) = self.product_accessor_target(cv, *n) {
                    let qe = self.accessor_receiver(cv, q, target);
                    return cv.expr(Expr::Select(qe, target));
                }
                if let Some(acc) = self.accessor_of(cv, t) {
                    if let Some(e) = self.conv_accessor(cv, acc) {
                        return e;
                    }
                }
                // `$anon.this.C` for the companion of a case class of the local class being
                // converted: the companion the typer derives, by its name.
                let local_this = match &q.kind {
                    TermKind::QualThis(t) => matches!(t, TType::LocalType(addr, _) if cv.local_classes.contains(addr)) || self.this_of_local_class(cv, t),
                    TermKind::Path(t) => self.this_of_local_class(cv, t),
                    _ => false,
                };
                if local_this {
                    let name = self.conv_name(cv, *n);
                    if cv.local_modules.values().any(|&m| m == name) {
                        return cv.expr(Expr::Ident(name));
                    }
                }
                // `C.this.super$pkg$Trait$$m`, the accessor scalac writes for `super.m` in a
                // trait, is the super call itself.
                if let Some(member) = self.super_accessor_member(cv, *n) {
                    let sup = cv.expr(Expr::Super(names::EMPTY));
                    return cv.expr(Expr::Select(sup, member));
                }
                // `scala.math.package.min`: the package object's member is the package's, as
                // the loader spills it and the std defines it; so is a member of `Predef` where
                // the std stands for it (`Predef.identity` of an identifier `Predef`).
                let predef = match &q.kind {
                    TermKind::Ident(_, t) | TermKind::Path(t) if self.is_unloaded_predef(cv, t) => match t {
                        TType::TermRef(pkg, _) => Some(self.conv_path(cv, pkg)),
                        _ => None,
                    },
                    _ => None,
                };
                let spilled = if predef.is_none() { self.package_object_owner(cv, q) } else { None };
                let qe = match predef.or(spilled) {
                    Some(pkg) => pkg,
                    None => match self.product_wrapped_array(cv, q) {
                        Some(array) => {
                            let e = self.conv_expr(cv, array);
                            self.adapted(cv, e, "an array wrapped for a member, the array the source selected on");
                            e
                        }
                        None => self.conv_expr(cv, q),
                    },
                };
                let name = self.select_name(cv, *n);
                if predef.is_some() && name == names::CLASS_OF && !self.loaded.as_ref().map_or(false, |l| l.scala_library) {
                    return cv.expr(Expr::Ident(name));
                }
                let name = if predef.is_some() && self.name_str(name) == "$conforms" { self.interner.intern("conforms") } else { name };
                // The declaration a `SELECTin` names, kept for the typer, where the qualifier is
                // the one the pickle selects on.
                let unary = self.name_ref(name).strip_prefix("unary_").filter(|op| !op.is_empty()).map(str::to_string);
                match unary {
                    Some(op) => {
                        let op = self.interner.intern(&op);
                        cv.expr(Expr::Prefix(op, qe))
                    }
                    None => {
                        let e = cv.expr(Expr::Select(qe, name));
                        if predef.is_none() && spilled.is_none() {
                            self.keep_declaration(cv, t, e);
                        }
                        e
                    }
                }
            }
            TermKind::QualThis(t) => self.conv_this(cv, t),
            TermKind::New(tpt) => {
                if let TType::LocalType(addr, _) = tpt {
                    if let Some(&d) = cv.anon_defs.get(addr) {
                        return cv.expr(Expr::NewAnon(d));
                    }
                }
                let ty = self.conv_new_type(cv, tpt, &[], false);
                cv.expr(Expr::New(ty, ListRef::EMPTY))
            }
            TermKind::Throw(e) => {
                let inner = self.conv_expr(cv, e);
                cv.expr(Expr::Throw(inner))
            }
            TermKind::NamedArg(n, v) => {
                let value = self.conv_expr(cv, v);
                let name = self.conv_name(cv, *n);
                cv.expr(Expr::NamedArg(name, value))
            }
            TermKind::Apply(f, args) if args.is_empty() && self.product_auto_applied(cv, t, f) => {
                let e = self.conv_expr(cv, f);
                self.adapted(cv, e, "Java's `()` the source did not write, left out");
                e
            }
            TermKind::Apply(f, args) => match self.conv_quoted(cv, f, args).or_else(|| self.conv_interpolation(cv, f, args)).or_else(|| self.conv_accessor_assign(cv, f, args)) {
                Some(e) => e,
                None => self.conv_apply(cv, f, args),
            },
            TermKind::Quote { body, .. } => {
                let b = self.conv_expr(cv, body);
                cv.expr(Expr::Quote(b))
            }
            TermKind::Splice { expr, .. } => self.conv_splice(cv, expr),
            TermKind::TypeApply(f, targs, inferred) => {
                if let Some(e) = self.conv_pattern_hole(cv, f, targs) {
                    return e;
                }
                if let Some(target) = self.companion_mirror(cv, t) {
                    return self.summon_mirror(cv, target);
                }
                // `q.ext[X](recv)[Y]`, an extension method with no clause after its own type
                // arguments, called through its owner: `q.ext[X, Y](recv)` here.
                if let Some((g, ext_targs, clauses)) = extension_clauses(f) {
                    if !self.selects_named(cv, g, "apply") {
                        return self.conv_extension_call(cv, g, ext_targs, targs, &clauses);
                    }
                }
                let fe = self.conv_expr(cv, f);
                // A type argument that names a captured wildcard (`slice.T`, `Array[?]#T`) is
                // inferred instead.
                if targs.iter().any(|a| self.mentions_wildcard_capture(cv, a)) {
                    return fe;
                }
                let items: Vec<TyExprId> = targs.iter().map(|a| self.conv_type_arg(cv, a)).collect();
                // A projection the loader could not read (`Array[?]#T`) leaves the arguments to
                // inference; one it reads as the wildcard's bound (`EndpointInput[?]#T`) stays.
                if targs.iter().any(|a| self.projects_wildcard(a)) && items.iter().any(|&t| matches!(cv.ast.ty(t), TyExpr::Resolved(r) if self.types.contains_error(r))) {
                    return fe;
                }
                let l = push_list(&mut cv.ast.ty_lists, &items);
                if *inferred {
                    cv.ast.inferred_type_lists.push(l.start);
                }
                cv.expr(Expr::TypeApply(fe, l))
            }
            TermKind::Super(_, mixin) => {
                let name = match mixin {
                    Some(TType::TypeRef(_, n)) => self.conv_name(cv, *n),
                    _ => names::EMPTY,
                };
                cv.expr(Expr::Super(name))
            }
            TermKind::Typed(e, ty, inferred) => {
                let inner = self.conv_expr(cv, e);
                let tasty = cv.tasty.clone();
                let t = match repeated_element(&tasty, ty) {
                    Some(elem) => {
                        let et = self.conv_type(cv, elem);
                        cv.ty(TyExpr::Repeated(et))
                    }
                    None => {
                        let t = self.conv_type(cv, ty);
                        if *inferred {
                            cv.mark_inferred(t);
                        }
                        t
                    }
                };
                cv.expr(Expr::Typed(inner, t))
            }
            TermKind::Assign(l, r) => {
                let (a, mut b) = (self.conv_expr(cv, l), self.conv_expr(cv, r));
                if let TermKind::Path(TType::LocalTerm(addr, _)) = &l.kind {
                    if let Some(&ty) = cv.cast_locals.get(addr) {
                        b = self.cast_to(cv, b, ty);
                    }
                }
                cv.expr(Expr::Assign(a, b))
            }
            TermKind::Block(stats, expr) => self.conv_block(cv, stats, expr),
            TermKind::If { inline, cond, then, els } => {
                let c = self.conv_expr(cv, cond);
                let t = self.conv_expr(cv, then);
                // A product's `if` without `else`: the `()` the writer puts in its place has no
                // span of the source (an explicit `else ()` has its own), and reads as no `else`,
                // as the whole program types it.
                let e = if cv.product && self.synthetic_unit(cv, els) { None } else { Some(self.conv_expr(cv, els)) };
                cv.expr(if *inline { Expr::InlineIf(c, t, e) } else { Expr::If(c, t, e) })
            }
            TermKind::Match { kind, selector, cases } => match (kind, selector) {
                (MatchKind::Implicit, _) => self.conv_summon_from(cv, cases),
                (MatchKind::Sub, _) | (_, None) => cv.refuse("a match of that kind"),
                (kind, Some(s)) => {
                    let scrut = self.conv_expr(cv, s);
                    let outer = std::mem::replace(&mut cv.in_inline_match, *kind == MatchKind::Inline);
                    let l = self.conv_cases(cv, cases);
                    cv.in_inline_match = outer;
                    cv.expr(if *kind == MatchKind::Inline { Expr::InlineMatch(scrut, l) } else { Expr::Match(scrut, l) })
                }
            },
            TermKind::Try { body, cases, finalizer } => {
                let b = self.conv_expr(cv, body);
                let cs = self.conv_cases(cv, cases);
                let f = finalizer.as_ref().map(|f| self.conv_expr(cv, f));
                cv.ast.tries.push(TryExpr { body: b, cases: cs, handler: None, finalizer: f });
                cv.expr(Expr::Try(cv.ast.tries.len() as u32 - 1))
            }
            TermKind::Return { expr, .. } => {
                let v = expr.as_ref().map(|e| self.conv_expr(cv, e));
                cv.expr(Expr::Return(v))
            }
            TermKind::While(c, b) => {
                let (x, y) = (self.conv_expr(cv, c), self.conv_expr(cv, b));
                cv.expr(Expr::While(x, y))
            }
            TermKind::Inlined { expansion, call, bindings, .. } if self.product_summoned(cv, expansion, call.as_deref(), bindings).is_some() => {
                let arg = self.product_summoned(cv, expansion, call.as_deref(), bindings).unwrap();
                if let Some(target) = self.companion_mirror(cv, arg) {
                    return self.summon_mirror(cv, target);
                }
                let a = self.conv_expr(cv, arg);
                let mut summon = cv.expr(Expr::Ident(names::ROOT));
                for seg in ["scala", "summon"] {
                    let n = self.interner.intern(seg);
                    summon = cv.expr(Expr::Select(summon, n));
                }
                let l = push_list(&mut cv.ast.expr_lists, &[a]);
                let e = cv.expr(Expr::UsingApply(summon, l));
                self.adapted(cv, e, "scala-library's `summon` expanded, the std's called");
                e
            }
            TermKind::Inlined { .. } if cv.replay_unresolved.contains_key(&t.at) => {
                let callee = cv.replay_unresolved[&t.at].clone();
                cv.refuse(format!("an expansion of {callee}, which the products' reader finds no one method of"))
            }
            TermKind::Inlined { expansion, call_at, bindings, .. } => {
                let mark = cv.scope.len();
                let mut stmts = Vec::new();
                // A binding shadows a binder in scope that a later binding or the expansion still
                // names (an argument read from the site's own `x` beside the proxy `x`).
                if self.converts_product(cv) && !cv.scope.is_empty() {
                    for (i, s) in bindings.iter().enumerate() {
                        let Stat::Val(sig, rhs) = s else { continue };
                        let mut within: Vec<&Term> = rhs.iter().collect();
                        within.extend(bindings[i + 1..].iter().filter_map(|s| match s {
                            Stat::Val(_, Some(t)) | Stat::Def(_, Some(t)) | Stat::Expr(t) => Some(t),
                            _ => None,
                        }));
                        within.push(expansion);
                        self.note_shadowing(cv, &[(sig.addr, sig.name)], &within, &[&sig.ret]);
                    }
                }
                self.hold_lambda_defs(cv, bindings);
                for s in bindings {
                    if !matches!(s, Stat::Def(sig, Some(_)) if cv.lambda_targets.contains(&sig.addr)) {
                        self.conv_stat(cv, s, &mut stmts);
                    }
                }
                let bound = stmts.len() as u32;
                let e = self.conv_expr(cv, expansion);
                // A product's expansion without bindings is the expansion itself, as the whole
                // program types it.
                if bound == 0 && self.converts_product(cv) {
                    cv.scope.truncate(mark);
                    cv.reader().traces.entry(e).or_insert(crate::ast::InlinedTrace { call_at: *call_at, bindings: 0 });
                    if let Some(&callee) = cv.replay_callees.get(&t.at) {
                        cv.reader().replay.expansions.insert(e, callee);
                    }
                    return e;
                }
                stmts.push(Stmt::Expr(e));
                let l = push_list(&mut cv.ast.stmts, &stmts);
                let block = cv.expr(Expr::Block(l));
                cv.scope.truncate(mark);
                let trace = crate::ast::InlinedTrace { call_at: *call_at, bindings: bound };
                cv.reader().traces.insert(block, trace);
                if let Some(&callee) = cv.replay_callees.get(&t.at) {
                    cv.reader().replay.expansions.insert(block, callee);
                }
                if super::declared::dump_path().is_some() {
                    self.dump_trace(cv, trace);
                }
                block
            }
            TermKind::Lambda(meth, sam) => {
                let held = match &meth.kind {
                    TermKind::Path(TType::LocalTerm(addr, _)) => cv.lambda_defs.remove(addr),
                    _ => None,
                };
                match held {
                    Some((sig, rhs)) => self.conv_lambda(cv, &sig, &rhs, sam.as_ref()),
                    None => cv.refuse("a closure outside the block that defines its method"),
                }
            }
            // A hole of a quote pattern, `$x` or `${p}`; a higher-order one, `$f(y)`, has no form
            // the typer types.
            TermKind::SplicePattern { pat, targs, args, .. } if targs.is_empty() && args.is_empty() => {
                let p = self.conv_pat(cv, pat);
                cv.expr(Expr::SplicePat(p))
            }
            TermKind::SplicePattern { .. } => cv.refuse("a higher-order splice pattern"),
            TermKind::Elided(_) => match self.withheld(cv, t.at) {
                Some(msg) => {
                    let s = cv.ast.add_str(msg);
                    cv.expr(Expr::Withheld(s))
                }
                None => cv.refuse(format!("{}", tag_name(t))),
            },
            _ => cv.refuse(format!("{}", tag_name(t))),
        }
    }

    /// The failure of a build that needs the body the product's pickle withholds at `addr`
    /// (its `TeqOrigins` kind 10): the definition, the module, the reason and the part of the
    /// writer that writes it. `None` for a pickle that records no reason, a
    /// scalac outline's.
    fn withheld(&self, cv: &Conv, addr: Addr) -> Option<String> {
        let loaded = self.loaded.as_ref()?;
        let file = loaded.file(cv.file);
        let crate::tasty::origins::Found::Read(o) = &file.provenance else { return None };
        let i = o.withheld.binary_search_by_key(&addr, |&(a, _)| a).ok()?;
        let reason = &o.withheld[i].1;
        let part = match reason.as_str() {
            "elided: inline method" | "elided: macro" | "elided: quote or splice" | "elided: library transparent expansion" | "elided: default of a held inline body" | "elided: a macro-made class's members, part 3's" => {
                "a later step of the TASTy writer, the one for the inline methods' and macros' bodies,"
            }
            _ => "the TASTy writer's queue of the shapes it withholds",
        };
        let module = &loaded.cp.paths[file.cp.jar as usize];
        Some(format!("{} is withheld from the products of {} ({}): {} writes it", cv.definition, module, reason, part))
    }

    /// The class an `INLINED`'s call names: after `PostTyper`, a reference to the top-level
    /// class of the inlined method (`Inlines.inlineCallTrace`), an identifier of its type or,
    /// for a macro, a selection of it on its package.
    fn inlined_origin(&mut self, cv: &Conv, call: &Term) -> Option<ClassId> {
        let t = match &call.kind {
            TermKind::Path(t) | TermKind::Ident(_, t) => t.clone(),
            TermKind::Select(q, n) => match &q.kind {
                TermKind::Path(p) => TType::TypeRef(Box::new(p.clone()), *n),
                _ => return None,
            },
            _ => return None,
        };
        if self.mentions_local_type(cv, &t) {
            return None;
        }
        let mut cx = super::MapCx::new(cv.file);
        let ctor = self.map_type_ctor(&mut cx, &t);
        match self.types.get(ctor) {
            Type::Class(k, _) | Type::Ctor(k) => Some(k),
            _ => None,
        }
    }

    /// Keeps the declaration a `SELECTin` names beside `e`, the selection or the operator made
    /// of it, for the typer to resolve.
    fn keep_declaration(&mut self, cv: &mut Conv, t: &Term, e: ExprId) {
        let TermKind::SelectIn(_, n, owner, owner_at) = &t.kind else { return };
        let Some(d) = self.declared_in(cv, *n, *owner_at) else { return };
        cv.reader().decls.insert(e, d);
        if super::declared::dump_path().is_some() {
            let spelled = crate::tasty::terms::qualified_name(&cv.tasty, &Decoder::new(&cv.tasty), owner);
            cv.reader().pickled_owners.insert(e, spelled);
        }
    }

    /// Records a std adaptation of the call converted into `e` (A2 to A9).
    fn adapted(&mut self, cv: &mut Conv, e: ExprId, what: &'static str) {
        cv.reader().adaptations.push((e, what));
        if super::declared::dump_path().is_some() {
            let member = cv.text.get(cv.span.start as usize..cv.span.end as usize).unwrap_or("").to_string();
            let class = cv.dump_class.clone().unwrap_or_default();
            let position = self.tasty_place(cv.file, cv.at).listed();
            super::declared::dump_line(format!("adapted\t{}\t{}\t{}\t{}", class, member, position, what));
        }
    }

    /// The bounds the `BIND`s of a typed pattern's type state for the case's variables, where
    /// they say more than `>: Nothing <: Any`: `Box[? >: String]`'s capture, whose bound
    /// neither the class's parameter nor the scrutinee gives. A bound over an outer case's
    /// variable, a type parameter or a path is converted as the body's types are.
    /// The bounds of a polymorphic function's or literal's type parameters as `Ast::lambda_bounds`
    /// lists them, `scala.Nothing` and `scala.Any` none; None where none has one. The
    /// parameters are in scope.
    fn conv_lambda_bounds(&mut self, cv: &mut Conv, params: &[TParam]) -> Option<ListRef> {
        let scala = |cv: &Conv, t: &TType, which: &str| {
            matches!(t, TType::TypeRef(prefix, n) if cv.tasty.simple(*n) == Some(which)
                && matches!(&**prefix, TType::Package(p) if cv.tasty.simple(*p) == Some("scala")))
        };
        let mut ids = Vec::with_capacity(2 * params.len());
        let mut any = false;
        for p in params {
            let (lo, hi) = match &p.info {
                TType::Bounds(lo, hi) => ((!scala(cv, lo, "Nothing")).then(|| (**lo).clone()), (!scala(cv, hi, "Any")).then(|| (**hi).clone())),
                _ => (None, None),
            };
            any |= lo.is_some() || hi.is_some();
            ids.push(lo.map_or(crate::ast::NO_BOUND, |t| self.conv_type(cv, &t)));
            ids.push(hi.map_or(crate::ast::NO_BOUND, |t| self.conv_type(cv, &t)));
        }
        any.then(|| push_list(&mut cv.ast.ty_lists, &ids))
    }

    fn binder_bounds(&mut self, cv: &mut Conv, ty: &TType) -> Vec<(Name, Option<TyExprId>, Option<TyExprId>)> {
        let mut binds = Vec::new();
        type_binds(&cv.tasty, ty, &mut binds);
        let mut out = Vec::new();
        for addr in binds {
            let Some(&name) = cv.local_types.get(&addr).filter(|_| cv.named_pattern_vars.contains_key(&addr)) else { continue };
            let Some(TType::Bounds(lo, hi)) = Decoder::new(&cv.tasty).bind_type(addr) else { continue };
            // `scala.Any` and `scala.Nothing` by their package: a class of the program may be
            // named so (`Box[? <: custom.Any]`).
            let scala = |t: &TType, which: &str| {
                matches!(t, TType::TypeRef(prefix, n) if cv.tasty.simple(*n) == Some(which)
                    && matches!(&**prefix, TType::Package(p) if cv.tasty.simple(*p) == Some("scala")))
            };
            let lo = (!scala(&lo, "Nothing")).then_some(lo);
            let hi = (!scala(&hi, "Any")).then_some(hi);
            if lo.is_none() && hi.is_none() {
                continue;
            }
            let lo = lo.map(|t| self.conv_type(cv, &t));
            let hi = hi.map(|t| self.conv_type(cv, &t));
            out.push((name, lo, hi));
        }
        out
    }

    /// The listing's line of a pattern's type variable: its name and the bounds its `BIND` states.
    fn dump_binder(&mut self, cv: &mut Conv, addr: Addr) {
        let member = cv.text.get(cv.span.start as usize..cv.span.end as usize).unwrap_or("").to_string();
        let class = cv.dump_class.clone().unwrap_or_default();
        let decoder = Decoder::new(&cv.tasty);
        let name = decoder.name_at(addr).map(|n| cv.tasty.name(n)).unwrap_or_default();
        let bounds = decoder.bind_type(addr).map(|b| crate::tasty::show::Printer::new(&cv.tasty, false).ty(&b)).unwrap_or_default();
        let position = self.tasty_place(cv.file, addr).listed();
        super::declared::dump_line(format!("binder\t{}\t{}\t{}\t{}\t{}", class, member, position, name, bounds));
    }

    fn dump_trace(&mut self, cv: &mut Conv, trace: crate::ast::InlinedTrace) {
        let member = cv.text.get(cv.span.start as usize..cv.span.end as usize).unwrap_or("").to_string();
        let origin = match trace.call_at {
            None => "from the caller's scope".to_string(),
            Some(at) => {
                let tasty = cv.tasty.clone();
                let mut decoder = Decoder::new(&tasty);
                let call = crate::tasty::terms::TermDecoder::new(&mut decoder).term_at(at);
                match self.inlined_origin(cv, &call) {
                    Some(k) => format!("from {}", self.class_path(k)),
                    None => "from a call of another shape".to_string(),
                }
            }
        };
        let class = cv.dump_class.clone().unwrap_or_default();
        let n = cv.reader().traces.len();
        let position = self.tasty_place(cv.file, cv.at).listed();
        super::declared::dump_line(format!("inlined\t{}\t{}\t{}\t#{}\t{}\t{} bindings", class, member, position, n, origin, trace.bindings));
    }

    /// `inline$x`: the accessor scalac made for a private member an inline body reaches,
    /// named by a selection, a path or the address of its definition.
    fn accessor_of(&mut self, cv: &mut Conv, term: &Term) -> Option<Accessor> {
        let tasty = cv.tasty.clone();
        let (prefix, name, addr): (Option<TType>, Option<u32>, Option<Addr>) = match &term.kind {
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) => {
                let prefix = match &q.kind {
                    TermKind::QualThis(t) => TType::This(Box::new(t.clone())),
                    TermKind::Path(t) => t.clone(),
                    _ => self.accessor_path(cv, q)?,
                };
                (Some(prefix), Some(*n), None)
            }
            TermKind::Path(TType::TermRef(prefix, n)) => (Some((**prefix).clone()), Some(*n), None),
            TermKind::Path(TType::LocalTerm(addr, prefix)) => (prefix.as_deref().cloned(), None, Some(*addr)),
            _ => return None,
        };
        // An accessor by its derived name (`INLINEACCESSOR`), not by a simple name's spelling.
        let accessor = match (name, addr) {
            (Some(n), _) => tasty.is_inline_accessor(n),
            (None, Some(a)) => Decoder::new(&tasty).name_at(a).map_or(false, |n| tasty.is_inline_accessor(n)),
            _ => return None,
        };
        if !accessor {
            return None;
        }
        // The prefix may be an accessor itself (`inline$IsUnionOf.inline$singleton`).
        let prefix = match prefix {
            Some(p) if self.names_accessor(cv, &p) => Some(self.accessor_path(cv, &Term::new(cv.at, TermKind::Path(p)))?),
            other => other,
        };
        let class = match &prefix {
            Some(TType::This(inner)) => self.this_class_of(cv, inner)?,
            Some(other) => match self.static_path_scope(cv, other)? {
                PathScope::Module(c) => c,
                PathScope::Pkg(_) => return None,
            },
            None => cv.this_chain.last().copied().filter(|&c| c != LOCAL_CLASS)?,
        };
        let lc = *self.loaded.as_ref()?.classes.get(&class)?;
        if lc.file != cv.file {
            return None;
        }
        let def = self.decoded_class(lc.file, lc.addr);
        let (sig, rhs) = match addr {
            Some(a) => {
                let mut decoder = Decoder::new(&tasty);
                let mut terms = TermDecoder::new(&mut decoder);
                terms.def_with_body(a)
            }
            None => def.template.stats.iter().find_map(|s| match s {
                // The same name is one entry of the file's name table.
                Stat::Def(sig, rhs) if name.map(|n| tasty.source_name(n)) == Some(tasty.source_name(sig.name)) => Some((sig.clone(), rhs.clone())),
                _ => None,
            })?,
        };
        Some(Accessor { class, def, sig, rhs })
    }

    /// Whether a path names an `inline$x` accessor rather than a local or an entered member.
    fn names_accessor(&self, cv: &Conv, t: &TType) -> bool {
        let tasty = &cv.tasty;
        match t {
            TType::TermRef(_, n) => tasty.is_inline_accessor(*n),
            TType::LocalTerm(addr, _) => !cv.locals.contains_key(addr) && Decoder::new(tasty).name_at(*addr).map_or(false, |n| tasty.is_inline_accessor(n)),
            _ => false,
        }
    }

    /// The object a parameterless accessor forwards to (`inline$IsUnionOf`), as the prefix of
    /// a further selection.
    fn accessor_path(&mut self, cv: &mut Conv, term: &Term) -> Option<TType> {
        let acc = self.accessor_of(cv, term)?;
        if takes_terms(&acc.sig) {
            return None;
        }
        match acc.rhs.map(|t| t.kind) {
            Some(TermKind::Path(t)) => Some(t),
            _ => None,
        }
    }

    /// The member of `class` an accessor's right-hand side selects on `this`, by name.
    fn accessor_own_member(&mut self, cv: &mut Conv, class: ClassId, term: &Term) -> Option<String> {
        let tasty = cv.tasty.clone();
        match &term.kind {
            TermKind::Path(TType::TermRef(prefix, m)) if matches!(&**prefix, TType::This(inner) if self.this_class_of(cv, inner) == Some(class)) => {
                Some(tasty.name(tasty.source_name(*m)))
            }
            TermKind::Select(q, m) | TermKind::SelectIn(q, m, ..) if matches!(&q.kind, TermKind::QualThis(inner) if self.this_class_of(cv, inner) == Some(class)) => {
                Some(tasty.name(tasty.source_name(*m)))
            }
            _ => None,
        }
    }

    /// A parameterless accessor stands for the private member or object it forwards to, one
    /// with parameters for the private method itself; a private member is entered here.
    fn conv_accessor(&mut self, cv: &mut Conv, acc: Accessor) -> Option<ExprId> {
        let tasty = cv.tasty.clone();
        let text = tasty.name(tasty.source_name(acc.sig.name));
        let target = text.strip_prefix("inline$")?;
        let member = if takes_terms(&acc.sig) {
            target.rsplit("$$").next().unwrap_or(target).to_string()
        } else {
            let forwarded = match acc.rhs.as_ref().map(|t| (t, &t.kind)) {
                // A polymorphic accessor forwards its type arguments (`inline$m$default$2[A,
                // B]` for `m$default$2[A, B]`): the member takes the call's.
                Some((_, TermKind::TypeApply(inner, ..))) => self.accessor_own_member(cv, acc.class, inner),
                Some((rhs, _)) => self.accessor_own_member(cv, acc.class, rhs),
                None => None,
            };
            match (forwarded, acc.rhs.as_ref().map(|t| (t, &t.kind))) {
                (Some(m), _) => m,
                // The field of a constructor parameter (`class C(using config: Configuration)`)
                // is private to the jar's class in link mode, where the accessor reads it.
                (None, Some((_, TermKind::Path(TType::LocalTerm(..))))) if self.link_mode() => {
                    let sym = self.member_sym(cv, acc.class, &acc.sig)?;
                    return Some(cv.expr(Expr::SymRef(sym)));
                }
                (None, Some((rhs, _))) => return Some(self.conv_expr(cv, rhs)),
                (None, None) => return None,
            }
        };
        let target_sig = acc.def.template.stats.iter().find_map(|s| match s {
            Stat::Def(sig, _) | Stat::Val(sig, _) if tasty.name(tasty.source_name(sig.name)) == member => Some(sig.clone()),
            _ => None,
        })?;
        // In link mode the output runs beside the jar, whose private member the JVM keeps to
        // its class: the call goes through the accessor, public in the class file.
        let called = if self.link_mode() { &acc.sig } else { &target_sig };
        let sym = self.member_sym(cv, acc.class, called)?;
        Some(cv.expr(Expr::SymRef(sym)))
    }

    /// `quote`, `splice` or `nestedSplice` when `f` names that method of
    /// `scala.quoted.runtime.Expr`, which is how a TASTy file before 28.4 spells quotes.
    /// The type of a splice, `nestedSplice[T](q)(f)` or a `SPLICE` node.
    fn spliced_type<'t>(&self, cv: &Conv, a: &'t Term) -> Option<&'t TType> {
        if let TermKind::Splice { ty, .. } = &a.kind {
            return Some(ty);
        }
        let mut head = a;
        while let TermKind::Apply(f, _) = &head.kind {
            head = f;
        }
        let TermKind::TypeApply(_, targs, ..) = &head.kind else { return None };
        match (self.quoted_runtime_op(cv, head), targs.as_slice()) {
            (Some("splice" | "nestedSplice"), [t]) => Some(t),
            _ => None,
        }
    }

    fn quoted_runtime_op(&self, cv: &Conv, f: &Term) -> Option<&'static str> {
        let head = match &f.kind {
            TermKind::TypeApply(inner, ..) => &**inner,
            _ => f,
        };
        let (qual, n) = match &head.kind {
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) => match &q.kind {
                TermKind::Path(t) | TermKind::QualThis(t) => (t, *n),
                _ => return None,
            },
            TermKind::Path(TType::TermRef(q, n)) => (&**q, *n),
            _ => return None,
        };
        let class = match qual {
            TType::This(inner) => &**inner,
            other => other,
        };
        let (prefix, cn) = match class {
            TType::TypeRef(p, cn) | TType::TermRef(p, cn) => (&**p, *cn),
            _ => return None,
        };
        if cv.tasty.simple(cv.tasty.source_name(cn)).map(|s| s.trim_end_matches('$')) != Some("Expr") {
            return None;
        }
        let in_runtime = match prefix {
            TType::Package(p) => cv.tasty.name(*p) == "scala.quoted.runtime",
            TType::TermRef(p, rn) => matches!(&**p, TType::Package(pp) if cv.tasty.name(*pp) == "scala.quoted") && cv.tasty.simple(*rn) == Some("runtime"),
            _ => false,
        };
        if !in_runtime {
            return None;
        }
        match cv.tasty.simple(cv.tasty.source_name(n))? {
            "quote" => Some("quote"),
            "splice" => Some("splice"),
            "nestedSplice" => Some("nestedSplice"),
            _ => None,
        }
    }

    /// A quote or splice written as a call of `scala.quoted.runtime.Expr`: `quote[T](body)`,
    /// applied to the `Quotes` in scope, `splice[T](fn)` and `nestedSplice[T](quotes)(fn)`.
    fn conv_quoted(&mut self, cv: &mut Conv, f: &Term, args: &[Term]) -> Option<ExprId> {
        if let (TermKind::Select(inner, n) | TermKind::SelectIn(inner, n, ..), [_]) = (&f.kind, args) {
            if cv.tasty.simple(cv.tasty.source_name(*n)) == Some("apply") {
                if let TermKind::Apply(g, inner_args) = &inner.kind {
                    if self.quoted_runtime_op(cv, g) == Some("quote") {
                        return self.conv_quoted(cv, g, inner_args);
                    }
                }
                // A later pickle's quote, `'{ e }.apply(q)`: the source quote `'{ e }`.
                if let TermKind::Quote { body, .. } = &inner.kind {
                    let b = self.conv_expr(cv, body);
                    return Some(cv.expr(Expr::Quote(b)));
                }
            }
        }
        if let TermKind::Apply(g, quotes) = &f.kind {
            if quotes.len() == 1 && args.len() == 1 && self.quoted_runtime_op(cv, g) == Some("nestedSplice") {
                return Some(self.conv_splice(cv, &args[0]));
            }
        }
        match (self.quoted_runtime_op(cv, f)?, args) {
            ("quote", [body]) => {
                let b = self.conv_expr(cv, body);
                Some(cv.expr(Expr::Quote(b)))
            }
            ("splice", [fun]) => Some(self.conv_splice(cv, fun)),
            _ => None,
        }
    }

    /// `${ body }` from the function scalac wrapped the body in, `(using q: Quotes) => body`.
    fn conv_splice(&mut self, cv: &mut Conv, fun: &Term) -> ExprId {
        let body = match &fun.kind {
            TermKind::Block(stats, expr) if stats.len() == 1 && matches!((**expr).kind, TermKind::Lambda(..)) => {
                if let Stat::Def(sig, Some(rhs)) = &stats[0] {
                    cv.lambda_defs.insert(sig.addr, (sig.clone(), rhs.clone()));
                }
                self.conv_splice_fun(cv, expr)
            }
            _ => self.conv_splice_fun(cv, fun),
        };
        cv.expr(Expr::Splice(body))
    }

    fn conv_splice_fun(&mut self, cv: &mut Conv, fun: &Term) -> ExprId {
        let held = match &fun.kind {
            TermKind::Lambda(meth, _) => match &meth.kind {
                TermKind::Path(TType::LocalTerm(addr, _)) => cv.lambda_defs.remove(addr),
                _ => None,
            },
            _ => None,
        };
        match held {
            Some((sig, rhs)) => {
                for clause in &sig.clauses {
                    let Clause::Terms(ps) = clause else { continue };
                    for p in ps {
                        cv.quotes_params.insert(p.addr, ());
                    }
                }
                self.conv_expr(cv, &rhs)
            }
            None => {
                let f = self.conv_expr(cv, fun);
                let q = self.summon_quotes(cv);
                let l = push_list(&mut cv.ast.expr_lists, &[q]);
                cv.expr(Expr::Apply(f, l))
            }
        }
    }

    /// The refinement `t` casts a product's companion to where it is the mirror of a case class
    /// as scalac's `Synthesizer` writes it (`companionPath`), `C.$asInstanceOf$[Mirror.Product {
    /// type MirroredMonoType = C; .. }]`: the product's companion holds none of the mirror members
    /// its pickle appends (`enter_loaded_class`), and the mirror is the one the typer synthesizes,
    /// as for a program's own.
    fn companion_mirror<'t>(&mut self, cv: &mut Conv, t: &'t Term) -> Option<&'t TType> {
        let TermKind::TypeApply(f, targs, _) = &t.kind else { return None };
        let (TermKind::SelectIn(q, n, ..) | TermKind::Select(q, n)) = &f.kind else { return None };
        if cv.tasty.name(cv.tasty.source_name(*n)) != "$asInstanceOf$" {
            return None;
        }
        let [target @ TType::Refined(base, members)] = &targs[..] else { return None };
        let product = matches!(&**base, TType::TypeRef(_, b) if cv.tasty.simple(*b) == Some("Product"));
        if !product || !members.iter().any(|(m, _)| cv.tasty.simple(*m) == Some("MirroredMonoType")) {
            return None;
        }
        let TermKind::Path(p) = &q.kind else { return None };
        let mut cx = super::MapCx::new(cv.file);
        let s = self.path_term(&mut cx, p)?;
        let SymKind::Object(m) = self.syms.sym(s).kind else { return None };
        self.syms.class(m).companion.filter(|_| self.is_product_class(m))?;
        Some(target)
    }

    /// `summon[T]` of the refinement a companion's mirror is cast to: the typer synthesizes it.
    /// The name as `Predef` brings it, scala-library's member there or the lean std's of `scala`.
    fn summon_mirror(&mut self, cv: &mut Conv, target: &TType) -> ExprId {
        let mut cx = super::MapCx::new(cv.file);
        let ty = self.map_type(&mut cx, target);
        let t = cv.ty(TyExpr::Resolved(ty));
        let summon = cv.expr(Expr::Ident(names::SUMMON));
        let l = push_list(&mut cv.ast.ty_lists, &[t]);
        cv.expr(Expr::TypeApply(summon, l))
    }

    /// `summon[Quotes]`: the `Quotes` a splice runs with.
    /// `summon[Type[t]]` for the `Type[t]` a quote type pattern bound: the typer synthesizes it
    /// from the type variable, as scalac's pattern does.
    fn summon_type_of(&mut self, cv: &mut Conv, var: Name) -> ExprId {
        match self.quoted_classes().ty {
            Some(c) => {
                let ctor = cv.ty(TyExpr::Resolved(self.types.mk(Type::Ctor(c))));
                let arg = cv.ty(TyExpr::Name(var));
                let al = push_list(&mut cv.ast.ty_lists, &[arg]);
                let t = cv.ty(TyExpr::Apply(ctor, al));
                let summon = cv.expr(Expr::Ident(names::SUMMON));
                let l = push_list(&mut cv.ast.ty_lists, &[t]);
                cv.expr(Expr::TypeApply(summon, l))
            }
            None => cv.refuse("a quote type pattern without scala.quoted in the standard library"),
        }
    }

    fn summon_quotes(&mut self, cv: &mut Conv) -> ExprId {
        match self.quoted_classes().quotes {
            Some(c) => {
                let ty = self.types.class(c, &[]);
                let t = cv.ty(TyExpr::Resolved(ty));
                let summon = cv.expr(Expr::Ident(names::SUMMON));
                let l = push_list(&mut cv.ast.ty_lists, &[t]);
                cv.expr(Expr::TypeApply(summon, l))
            }
            None => cv.refuse("a splice without scala.quoted in the standard library"),
        }
    }

    /// Whether a path is `scala.Predef` while no jar defines the object, so that its members
    /// are the std's at the level of package `scala`.
    /// `scala.Predef` where the std stands for it: under the lean std always (its wrappers are
    /// the std's own members on the wrapped types), otherwise when no jar defines the object.
    fn is_unloaded_predef(&mut self, cv: &Conv, t: &TType) -> bool {
        self.is_predef_path(cv, t) && (self.std_binds() || !matches!(self.pkg_term(self.b.scala_pkg, names::PREDEF), Some(r) if r.sym().is_some()))
    }

    /// `scala.Predef` as a path.
    fn is_predef_path(&self, cv: &Conv, t: &TType) -> bool {
        let TType::TermRef(pkg, obj) = t else { return false };
        let TType::Package(p) = &**pkg else { return false };
        cv.tasty.simple(*p) == Some("scala") && cv.tasty.simple(cv.tasty.source_name(*obj)) == Some("Predef")
    }

    /// `Predef.$conforms[T]`, the evidence scalac passes for a conversion that is the identity.
    fn is_conforms_evidence(&mut self, cv: &Conv, a: &Term) -> bool {
        let head = match &a.kind {
            TermKind::TypeApply(inner, ..) => &**inner,
            _ => a,
        };
        let (qual, n) = match &head.kind {
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) => match &q.kind {
                TermKind::Path(t) => (t, *n),
                _ => return false,
            },
            TermKind::Path(TType::TermRef(q, n)) => (&**q, *n),
            _ => return false,
        };
        cv.tasty.simple(cv.tasty.source_name(n)) == Some("$conforms") && self.is_unloaded_predef(cv, qual)
    }

    /// `scala.quoted.Type.of[T](quotes)`: the `Type` evidence scalac passes to a using clause.
    fn is_type_of_call(&self, cv: &Conv, a: &Term) -> bool {
        let TermKind::Apply(f, _) = &a.kind else { return false };
        let head = match &f.kind {
            TermKind::TypeApply(inner, ..) => &**inner,
            _ => f,
        };
        let (qual, n) = match &head.kind {
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) => match &q.kind {
                TermKind::Path(t) => (t, *n),
                _ => return false,
            },
            TermKind::Path(TType::TermRef(q, n)) => (&**q, *n),
            _ => return false,
        };
        let TType::TermRef(pkg, obj) = qual else { return false };
        let TType::Package(p) = &**pkg else { return false };
        cv.tasty.name(*p) == "scala.quoted" && cv.tasty.simple(cv.tasty.source_name(*obj)) == Some("Type") && cv.tasty.simple(cv.tasty.source_name(n)) == Some("of")
    }

    /// `ClassTag.apply[T](classOf[T])` or `ClassTag.Int`: the evidence scalac passed for an
    /// array's element type.
    fn is_class_tag_evidence(&mut self, cv: &Conv, a: &Term) -> bool {
        // A splice of a `ClassTag` into a quote (`Array(xs*)(${ ct })`).
        if let Some(t) = self.spliced_type(cv, a) {
            return self.is_class_tag_type(cv, t);
        }
        let param_type: Option<&TType> = match &a.kind {
            TermKind::Path(TType::LocalTerm(_, Some(ty))) => Some(ty),
            TermKind::Path(TType::LocalTerm(addr, None)) => cv.local_val_types.get(addr),
            TermKind::Ident(_, ty) => Some(ty),
            _ => None,
        };
        if param_type.is_some_and(|ty| self.is_class_tag_type(cv, ty)) {
            return true;
        }
        let head = match &a.kind {
            TermKind::Apply(f, _) => match &f.kind {
                TermKind::TypeApply(inner, ..) => &**inner,
                _ => f,
            },
            _ => a,
        };
        let (qual, n) = match &head.kind {
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) => match &q.kind {
                TermKind::Path(t) => (t, *n),
                // `_root_.scala.reflect.ClassTag.apply`, as a chain of selections.
                TermKind::Select(r, obj) | TermKind::SelectIn(r, obj, ..) => {
                    return cv.tasty.simple(cv.tasty.source_name(*obj)) == Some("ClassTag")
                        && matches!(&r.kind, TermKind::Select(s, reflect) if cv.tasty.simple(cv.tasty.source_name(*reflect)) == Some("reflect") && matches!(&s.kind, TermKind::Select(_, sc) if cv.tasty.simple(cv.tasty.source_name(*sc)) == Some("scala")));
                }
                _ => return false,
            },
            TermKind::Path(TType::TermRef(q, n)) => (&**q, *n),
            _ => return false,
        };
        let TType::TermRef(pkg, obj) = qual else { return false };
        let TType::Package(p) = &**pkg else { return false };
        let _ = n;
        cv.tasty.name(*p) == "scala.reflect" && cv.tasty.simple(cv.tasty.source_name(*obj)) == Some("ClassTag")
    }

    /// `scala.reflect.ClassTag[T]`, the type of an evidence parameter.
    fn is_class_tag_type(&self, cv: &Conv, ty: &TType) -> bool {
        let head = match ty {
            TType::Applied(f, _) => &**f,
            other => other,
        };
        let TType::TypeRef(prefix, n) = head else { return false };
        cv.tasty.simple(cv.tasty.source_name(*n)) == Some("ClassTag")
            && matches!(&**prefix, TType::Package(p) if cv.tasty.name(*p) == "scala.reflect")
    }

    /// Whether a call's receiver is a local of the type `ArrayOps`, which scalac binds the
    /// wrapper to where a call passes a default (`val $1 = intArrayOps(xs); $1.m(p, $1.m$default$2)`).
    fn on_array_ops_local(&self, cv: &Conv, f: &Term) -> bool {
        let head = match &f.kind {
            TermKind::TypeApply(inner, ..) => &**inner,
            _ => f,
        };
        let (TermKind::Select(q, _) | TermKind::SelectIn(q, _, ..)) = &head.kind else { return false };
        let ty = match &q.kind {
            TermKind::Path(TType::LocalTerm(_, Some(ty))) => ty,
            TermKind::Path(TType::LocalTerm(addr, None)) => match cv.local_val_types.get(addr) {
                Some(ty) => ty,
                None => return false,
            },
            _ => return false,
        };
        let head = match ty {
            TType::Applied(f, _) => &**f,
            other => other,
        };
        matches!(head, TType::TypeRef(_, n) if cv.tasty.simple(cv.tasty.source_name(*n)).map_or(false, |s| s.ends_with("ArrayOps")))
    }

    /// Whether a call's receiver is an array through one of `scala.Predef`'s `ArrayOps`
    /// wrappers that no jar defines, or that wrapper's `withFilter`.
    fn through_array_ops(&mut self, cv: &mut Conv, f: &Term) -> bool {
        let mut head = f;
        loop {
            match &head.kind {
                TermKind::Apply(g, _) | TermKind::TypeApply(g, ..) => head = g,
                _ => break,
            }
        }
        let (TermKind::Select(q, _) | TermKind::SelectIn(q, _, ..)) = &head.kind else { return false };
        let TermKind::Apply(w, _) = &q.kind else { return false };
        // `ArrayOps.withFilter(p)`, whose `map` and `flatMap` take the evidence as the array's do.
        if self.selects_named(cv, w, "withFilter") {
            return self.through_array_ops(cv, w);
        }
        let name = match &w.kind {
            TermKind::TypeApply(inner, ..) => match &inner.kind {
                TermKind::Select(_, n) | TermKind::SelectIn(_, n, ..) => *n,
                TermKind::Path(TType::TermRef(_, n)) => *n,
                _ => return false,
            },
            TermKind::Select(_, n) | TermKind::SelectIn(_, n, ..) => *n,
            TermKind::Path(TType::TermRef(_, n)) => *n,
            _ => return false,
        };
        cv.tasty.simple(cv.tasty.source_name(name)).map_or(false, |s| s.ends_with("ArrayOps")) && matches!(self.predef_wrapper(cv, w), Some(PredefWrapper::Identity))
    }

    /// The arguments of `recv.m(a, recv.m$default$2)` without the trailing ones that are the
    /// method's own defaults: the std's `m` of an array supplies its default itself, or has no
    /// such parameter and does what the default does (munit calls an array's `indexWhere` so).
    /// With `leading`, a getter applied to the earlier clauses' arguments counts too (a
    /// product's call, where a default the source left out is the getter's call), and the
    /// clause's arguments follow the earlier clauses' in the getters' numbering.
    fn without_own_defaults<'t>(&self, cv: &Conv, f: &Term, args: &'t [Term], leading: bool) -> &'t [Term] {
        let mut head = f;
        let mut before = 0;
        loop {
            match &head.kind {
                TermKind::TypeApply(inner, ..) => head = inner,
                TermKind::Apply(inner, earlier) if leading => {
                    before += earlier.len();
                    head = inner;
                }
                _ => break,
            }
        }
        let (TermKind::Select(_, n) | TermKind::SelectIn(_, n, ..)) = &head.kind else { return args };
        let callee = cv.tasty.source_name(*n);
        let mut end = args.len();
        while end > 0 {
            let mut a = &args[end - 1];
            loop {
                match &a.kind {
                    TermKind::Apply(g, inner) if inner.is_empty() || leading => a = g,
                    TermKind::TypeApply(g, ..) => a = g,
                    _ => break,
                }
            }
            let own = match &a.kind {
                TermKind::Select(_, d) | TermKind::SelectIn(_, d, ..) => match cv.tasty.names.get(cv.tasty.source_name(*d) as usize) {
                    Some(TName::DefaultGetter(u, index)) => *index as usize + 1 == before + end && cv.tasty.simple(*u).is_some() && cv.tasty.simple(*u) == cv.tasty.simple(callee),
                    _ => false,
                },
                _ => false,
            };
            if !own {
                break;
            }
            end -= 1;
        }
        &args[..end]
    }

    /// A product's call that passes its callee's own default getter where the source left an
    /// argument out before named ones (`c.copy(c.copy$default$1, balance = b, ..)`): the named
    /// arguments alone, as the source wrote them. `None` where an argument it passes is
    /// neither a default getter of its own nor named.
    fn named_without_defaults(&self, cv: &Conv, f: &Term, args: &[Term]) -> Option<Vec<Term>> {
        let trimmed = self.without_own_defaults(cv, f, args, true);
        if !args[..trimmed.len()].iter().any(|a| matches!(a.kind, TermKind::NamedArg(..))) {
            return None;
        }
        let mut kept = Vec::new();
        let mut dropped = false;
        for (i, a) in args.iter().enumerate() {
            match &a.kind {
                TermKind::NamedArg(..) => kept.push(a.clone()),
                // The argument at `i` is the callee's own default where the arguments up to it,
                // trimmed of the trailing ones that are, end at or before it.
                _ if self.without_own_defaults(cv, f, &args[..=i], true).len() <= i => dropped = true,
                _ => return None,
            }
        }
        dropped.then_some(kept)
    }

    /// Whether `f`, under its type application, selects the member `name`.
    fn selects_named(&self, cv: &Conv, f: &Term, name: &str) -> bool {
        let head = match &f.kind {
            TermKind::TypeApply(inner, ..) => &**inner,
            _ => f,
        };
        matches!(&head.kind, TermKind::Select(_, n) | TermKind::SelectIn(_, n, ..) if cv.tasty.simple(cv.tasty.source_name(*n)) == Some(name))
    }

    /// A member of the `Array` or `IArray` companion (`Array.apply`, `IArray.from`, the `IArray`
    /// extensions called through it), whose evidence the std does without; `ofDim` keeps its
    /// `ClassTag`, which chooses the array's zero.
    fn through_array_companion(&self, cv: &Conv, f: &Term) -> bool {
        let mut head = f;
        loop {
            match &head.kind {
                TermKind::Apply(g, _) | TermKind::TypeApply(g, ..) => head = g,
                _ => break,
            }
        }
        let (TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..)) = &head.kind else { return false };
        if cv.tasty.simple(cv.tasty.source_name(*n)) == Some("ofDim") {
            return false;
        }
        let object = match &q.kind {
            TermKind::Path(TType::TermRef(_, obj)) => *obj,
            TermKind::Select(_, obj) | TermKind::SelectIn(_, obj, ..) => *obj,
            _ => return false,
        };
        matches!(cv.tasty.simple(cv.tasty.source_name(object)), Some("Array" | "IArray"))
    }

    /// The array of a product's `wrapRefArray[T](xs).m`, which teq's writer makes of the source's
    /// `xs.m` where scala-library has the std's array member on an `ArraySeq`: the typer resolves
    /// `xs.m` against the std as the source's did.
    fn product_wrapped_array<'t>(&mut self, cv: &mut Conv, q: &'t Term) -> Option<&'t Term> {
        if !cv.product {
            return None;
        }
        let TermKind::Apply(w, args) = &q.kind else { return None };
        match args.as_slice() {
            [array] if matches!(self.predef_wrapper(cv, w), Some(PredefWrapper::ArrayToSeq)) => Some(array),
            _ => None,
        }
    }

    /// The implicit conversions of `scala.Predef` that a pickled body applies explicitly, when
    /// no jar defines the object: the std has their methods on the wrapped types themselves.
    fn predef_wrapper(&mut self, cv: &mut Conv, f: &Term) -> Option<PredefWrapper> {
        let head = match &f.kind {
            TermKind::TypeApply(inner, ..) => &**inner,
            _ => f,
        };
        let (qual, n) = match &head.kind {
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) => match &q.kind {
                TermKind::Path(t) => (t, *n),
                _ => return None,
            },
            TermKind::Path(TType::TermRef(q, n)) => (&**q, *n),
            _ => return None,
        };
        if !self.is_unloaded_predef(cv, qual) {
            return if self.extension_holder_given(cv, qual, n) { Some(PredefWrapper::Identity) } else { None };
        }
        let name = cv.tasty.simple(cv.tasty.source_name(n))?;
        match name {
            "augmentString" | "wrapString" | "ArrowAssoc" | "Ensuring" | "any2stringadd" | "$conforms" | "intWrapper" | "longWrapper" | "doubleWrapper" | "floatWrapper" | "charWrapper" | "booleanWrapper" | "byteWrapper" | "shortWrapper" => {
                Some(PredefWrapper::Identity)
            }
            _ if name.ends_with("ArrayOps") => Some(PredefWrapper::Identity),
            "genericWrapArray" => Some(PredefWrapper::ArrayToSeq),
            "copyArrayToImmutableIndexedSeq" => Some(PredefWrapper::ArrayCopyToSeq),
            _ if name.starts_with("wrap") && name.ends_with("Array") => Some(PredefWrapper::ArrayToSeq),
            _ => None,
        }
    }

    /// A given of the std standing for a Scala.js implicit class (`given AB2TA:
    /// ByteArrayOps.type`, `JSConverters.JSRichIterable`): an object holding the extension
    /// methods, which a body applies as the conversion and selects the method on. The wrapped
    /// value is the receiver of the std's extension.
    fn extension_holder_given(&mut self, cv: &mut Conv, qual: &TType, n: u32) -> bool {
        if !self.in_scalajs_layer(cv, qual) {
            return false;
        }
        let mut cx = super::MapCx::new(cv.file);
        let Some(sym) = self.path_term(&mut cx, &TType::TermRef(Box::new(qual.clone()), n)) else { return false };
        if self.syms.sym(sym).kind != SymKind::Given || self.in_jar(self.syms.sym(sym).file) {
            return false;
        }
        let ty = self.sig_of(sym).ret;
        match self.types.get(ty) {
            Type::Class(o, _) if self.syms.class(o).kind == ClassKind::Object && !self.syms.class(o).extensions.is_empty() => {
                if !cv.holder_objects.contains(&o) {
                    cv.holder_objects.push(o);
                }
                true
            }
            _ => false,
        }
    }

    /// Whether a qualifier lies under `scala.scalajs`, the packages of the std's facade layer:
    /// resolving any other object here would complete a jar's class before its time.
    fn in_scalajs_layer(&self, cv: &Conv, qual: &TType) -> bool {
        let mut t = qual;
        loop {
            match t {
                TType::TermRef(prefix, _) => t = prefix,
                TType::Package(p) => {
                    let name = cv.tasty.name(*p);
                    return name == "scala.scalajs" || name.starts_with("scala.scalajs.");
                }
                _ => return false,
            }
        }
    }

    /// The name of a selection: scalac's internal spellings of the casts read as the source
    /// ones, and a super accessor as the `super` selection it stands for.
    fn select_name(&mut self, cv: &Conv, n: u32) -> Name {
        let text = cv.tasty.name(cv.tasty.source_name(n));
        match text.as_str() {
            "$asInstanceOf$" => names::AS_INSTANCE_OF,
            "$isInstanceOf$" => names::IS_INSTANCE_OF,
            _ => self.interner.intern(&text),
        }
    }

    fn conv_arg(&mut self, cv: &mut Conv, a: &Term) -> ExprId {
        self.conv_expr(cv, a)
    }

    /// `StringContext.apply(parts*).s(args*)`, scalac's spelling of `s"..."` in a body, as the
    /// interpolation the typer treats natively; scala-library's own `s` is a stub.
    fn conv_interpolation(&mut self, cv: &mut Conv, f: &Term, args: &[Term]) -> Option<ExprId> {
        let (TermKind::Select(recv, method) | TermKind::SelectIn(recv, method, ..)) = &f.kind else { return None };
        let kind = match cv.tasty.simple(cv.tasty.source_name(*method))? {
            "s" => names::S_INTERP,
            "raw" => names::RAW_INTERP,
            _ => return None,
        };
        let TermKind::Apply(ctor, parts) = &recv.kind else { return None };
        let (TermKind::Select(context, apply) | TermKind::SelectIn(context, apply, ..)) = &ctor.kind else { return None };
        if cv.tasty.simple(cv.tasty.source_name(*apply)) != Some("apply") || !self.is_string_context(cv, context) {
            return None;
        }
        let (part_terms, arg_terms) = (repeated_items(parts)?, repeated_items(args)?);
        let mut strs = Vec::with_capacity(part_terms.len());
        for p in part_terms {
            let TermKind::Const(Const::Str(n)) = &p.kind else { return None };
            // The parts are pickled as written; `s` processes their escapes and `raw` only `$$`,
            // as the parser does for a source interpolation.
            let text = cv.tasty.name(*n);
            let text = if kind == names::S_INTERP {
                let mut errors = Vec::new();
                crate::parser::unescape(&text, true, &mut errors, cv.span)
            } else {
                text.replace("$$", "$")
            };
            strs.push(cv.ast.add_str(text));
        }
        let items: Vec<ExprId> = arg_terms.iter().map(|a| self.conv_expr(cv, a)).collect();
        let parts = push_list(&mut cv.ast.str_lists, &strs);
        let args = push_list(&mut cv.ast.expr_lists, &items);
        Some(cv.expr(Expr::Interp(kind, parts, args)))
    }

    /// `scala.StringContext`: a reference to the object, or the selections `_root_.scala.
    /// StringContext` an older compiler pickles.
    fn is_string_context(&self, cv: &Conv, t: &Term) -> bool {
        match &t.kind {
            TermKind::Path(TType::TermRef(pkg, obj)) => {
                cv.tasty.simple(cv.tasty.source_name(*obj)) == Some("StringContext")
                    && matches!(&**pkg, TType::Package(p) if cv.tasty.name(*p).rsplit('.').next() == Some("scala"))
            }
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) => {
                cv.tasty.simple(cv.tasty.source_name(*n)) == Some("StringContext")
                    && matches!(&q.kind, TermKind::Select(root, s) if cv.tasty.simple(cv.tasty.source_name(*s)) == Some("scala") && matches!(&root.kind, TermKind::Path(TType::Package(_))))
            }
            _ => false,
        }
    }

    /// `inline$FocusImpl$i1(monocle.internal.focus)`, the accessor scalac makes for a
    /// qualified-private object an inline body reaches through a package: its parameter is the
    /// package and its body selects the object on it, so the call is that selection.
    fn conv_package_accessor(&mut self, cv: &mut Conv, f: &Term, args: &[Term]) -> Option<ExprId> {
        let [arg] = args else { return None };
        let acc = self.accessor_of(cv, f)?;
        let param = acc.sig.clauses.iter().find_map(|cl| match cl {
            Clause::Terms(ps) if ps.len() == 1 => Some(ps[0].addr),
            _ => None,
        })?;
        let on_param = |t: &TType| matches!(t, TType::LocalTerm(a, _) if *a == param);
        let member = match acc.rhs.as_ref().map(|t| &t.kind) {
            Some(TermKind::Path(TType::TermRef(prefix, member))) if on_param(prefix) => *member,
            Some(TermKind::Select(q, member) | TermKind::SelectIn(q, member, ..)) if matches!(&q.kind, TermKind::Path(t) if on_param(t)) => *member,
            _ => return None,
        };
        let TermKind::Path(path @ TType::Package(_)) = &arg.kind else { return None };
        let q = self.conv_path(cv, path);
        let name = self.conv_name(cv, member);
        Some(cv.expr(Expr::Select(q, name)))
    }

    fn conv_apply(&mut self, cv: &mut Conv, f: &Term, args: &[Term]) -> ExprId {
        // Through an array's `ArrayOps`, whose std members lack some of scala-library's
        // parameters, and in a product's body, where the source left them out: elsewhere the
        // default getters stay, a JS facade's among them.
        let product = self.converts_product(cv);
        let named_only: Vec<Term>;
        let args = match product {
            true => match self.named_without_defaults(cv, f, args) {
                Some(named) => {
                    named_only = named;
                    &named_only[..]
                }
                None => self.without_own_defaults(cv, f, args, true),
            },
            false => {
                let trimmed = self.without_own_defaults(cv, f, args, false);
                if trimmed.len() < args.len() && (self.on_array_ops_local(cv, f) || self.through_array_ops(cv, f)) { trimmed } else { args }
            }
        };
        if let Some(e) = self.conv_package_accessor(cv, f, args) {
            return e;
        }
        // A companion's mirror passed to a using clause (`Schema.derived[C](C.$asInstanceOf$[..])`)
        // is left to the typer, which synthesizes it there as the program's own build does.
        if let [arg] = args {
            if self.companion_mirror(cv, arg).is_some() && self.fills_using_clause(cv, f) {
                return self.conv_expr(cv, f);
            }
        }
        // `IArray.apply[T](arr)(i)`, the extension called through its owner, whose name the
        // companion's `apply(xs*)` would take: the element of `arr`.
        if let (TermKind::Apply(g, recv), [index]) = (&f.kind, args) {
            let head = match &g.kind {
                TermKind::TypeApply(inner, ..) => &**inner,
                _ => g,
            };
            if let (TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..), [arr]) = (&head.kind, recv.as_slice()) {
                let object = match &q.kind {
                    TermKind::Path(TType::TermRef(_, obj)) => Some(*obj),
                    TermKind::Select(_, obj) | TermKind::SelectIn(_, obj, ..) => Some(*obj),
                    _ => None,
                };
                let vararg = matches!(&arr.kind, TermKind::Typed(inner, ..) if matches!((**inner).kind, TermKind::Repeated(..))) || matches!(&arr.kind, TermKind::Repeated(..));
                if !vararg
                    && !self.is_class_tag_evidence(cv, index)
                    && cv.tasty.simple(cv.tasty.source_name(*n)) == Some("apply")
                    && object.map_or(false, |o| cv.tasty.simple(cv.tasty.source_name(o)) == Some("IArray"))
                {
                    let receiver = self.conv_expr(cv, arr);
                    let i = self.conv_expr(cv, index);
                    let callee = cv.expr(Expr::Select(receiver, names::APPLY));
                    let l = push_list(&mut cv.ast.expr_lists, &[i]);
                    return cv.expr(Expr::Apply(callee, l));
                }
            }
        }
        // The holes of a quote pattern are bound in the order they are written: those of the
        // function (`when(hole1)` of `when(hole1)(hole2)`) come before its arguments'.
        let function_holes = if cv.pattern_holes.is_empty() { None } else {
            let k = count_pattern_holes(&cv.tasty, f).min(cv.pattern_holes.len());
            Some(cv.pattern_holes.drain(..k).collect::<Vec<PatId>>())
        };
        let widened = self.product_widened(cv, f);
        let mut items: Vec<ExprId> = Vec::with_capacity(args.len());
        for a in args {
            if let Some(c) = widened.then(|| self.char_widened_arg(cv, a)).flatten() {
                items.push(self.conv_expr(cv, c));
                continue;
            }
            match &a.kind {
                TermKind::Typed(inner, ..) if matches!((**inner).kind, TermKind::Repeated(..)) => {
                    if let TermKind::Repeated(_, elems) = &inner.kind {
                        for e in elems {
                            items.push(self.conv_expr(cv, e));
                        }
                    }
                }
                TermKind::Repeated(_, elems) => {
                    for e in elems {
                        items.push(self.conv_expr(cv, e));
                    }
                }
                _ => items.push(self.conv_expr(cv, a)),
            }
        }
        if let Some(front) = function_holes {
            for n in front.into_iter().rev() {
                cv.pattern_holes.push_front(n);
            }
        }
        if items.len() == 1 {
            if let Some(wrapper) = self.predef_wrapper(cv, f) {
                let what = match wrapper {
                    PredefWrapper::Identity => "a Predef wrapper applied by the pickle, the wrapped value",
                    PredefWrapper::ArrayToSeq | PredefWrapper::ArrayCopyToSeq => "a Predef array wrapper, ArraySeq.unsafeWrapArray",
                };
                self.adapted(cv, items[0], what);
                return match wrapper {
                    PredefWrapper::Identity => items[0],
                    // scala-library's `ArraySeq` over the array, not a copy, or over a copy.
                    PredefWrapper::ArrayToSeq | PredefWrapper::ArrayCopyToSeq => {
                        let mut path = cv.expr(Expr::Ident(names::ROOT));
                        for seg in ["scala", "ArraySeq", "unsafeWrapArray"] {
                            let n = self.interner.intern(seg);
                            path = cv.expr(Expr::Select(path, n));
                        }
                        let array = match wrapper {
                            PredefWrapper::ArrayCopyToSeq => {
                                let clone = self.interner.intern("clone");
                                let select = cv.expr(Expr::Select(items[0], clone));
                                let none = push_list(&mut cv.ast.expr_lists, &[]);
                                cv.expr(Expr::Apply(select, none))
                            }
                            _ => items[0],
                        };
                        let args = push_list(&mut cv.ast.expr_lists, &[array]);
                        cv.expr(Expr::Apply(path, args))
                    }
                };
            }
        }
        // `arr.map(f)(ClassTag)` through an `ArrayOps` wrapper, `IArray.from(xs)(ClassTag)` or
        // `Array(xs*)(ClassTag)`: the std's array members make the array without evidence. A
        // product's body keeps the clause, which the typer takes as the one written for the
        // evidence the std leaves out, for a pickle of the body to pass it again.
        if !args.is_empty() && args.iter().all(|a| self.is_class_tag_evidence(cv, a)) {
            let through_ops = self.through_array_ops(cv, f) || self.selects_named(cv, f, "toArray");
            if through_ops || self.through_array_companion(cv, f) {
                let e = match &f.kind {
                    TermKind::Apply(g, inner) if !through_ops => self.conv_apply(cv, g, inner),
                    _ => self.conv_expr(cv, f),
                };
                if self.converts_product(cv) {
                    let evidence: Vec<ExprId> = args.iter().map(|a| self.conv_expr(cv, a)).collect();
                    let l = push_list(&mut cv.ast.expr_lists, &evidence);
                    return cv.expr(Expr::UsingApply(e, l));
                }
                self.adapted(cv, e, "the ClassTag argument clause dropped");
                return e;
            }
        }
        // `xs.unzip[A1, A2](Predef.$conforms)`: the std's `unzip` and `unzip3`, of a collection of
        // pairs or triples, take neither.
        if let [a] = args {
            if (self.selects_named(cv, f, "unzip") || self.selects_named(cv, f, "unzip3")) && self.is_conforms_evidence(cv, a) {
                let head = match &f.kind {
                    TermKind::TypeApply(inner, ..) => &**inner,
                    _ => f,
                };
                let e = self.conv_expr(cv, head);
                self.adapted(cv, e, "the type arguments and the <:< argument clause dropped");
                return e;
            }
        }
        let l = push_list(&mut cv.ast.expr_lists, &items);
        // `new C[T](args)(using)`: the AST keeps the first argument list of a constructor call
        // and the typer supplies the using clauses again.
        if self.constructed_new(cv, f).is_some() && args.iter().all(|a| self.is_given_argument(cv, a)) {
            let new = self.conv_expr(cv, f);
            // After the `()` scalac puts before a leading using clause (`new C()(q)(a)`), the
            // list is the one written.
            if matches!(&f.kind, TermKind::Apply(_, first) if first.is_empty()) {
                return cv.expr(Expr::UsingApply(new, l));
            }
            // With its type arguments inferred again, the constructor takes the givens the
            // body passes, which the class path's definitions may be alone to provide.
            if matches!(&f.kind, TermKind::Apply(g, _) if matches!(&g.kind, TermKind::TypeApply(_, _, true))) {
                let passing = cv.expr(Expr::UsingApply(new, l));
                cv.ast.inferred_alternatives.push((new, passing));
            }
            return new;
        }
        let mut head = f;
        let mut targs: &[TType] = &[];
        let mut inferred = false;
        if let TermKind::TypeApply(inner, ta, i) = &head.kind {
            head = inner;
            targs = ta;
            inferred = *i;
        }
        // `q.ext(recv)[X](args)`, an extension method called through its owner with its type
        // arguments after the receiver clause, is `q.ext[X](recv)(args)` here; so is
        // `ext(using q)(recv)[X](args)` of an extension with a leading using clause.
        if let (TermKind::Apply(..), false) = (&head.kind, targs.is_empty()) {
            let mut ext_lists: Vec<&[Term]> = Vec::new();
            let mut g = head;
            while let TermKind::Apply(inner, list) = &g.kind {
                ext_lists.push(list);
                g = inner;
            }
            ext_lists.reverse();
            // `IArray.map[T](arr)[U](f)`: the extension's own type arguments come first.
            let (g, ext_targs): (&Term, &[TType]) = match &g.kind {
                TermKind::TypeApply(inner, ta, ..) if matches!(&inner.kind, TermKind::Select(..) | TermKind::SelectIn(..) | TermKind::Path(_)) => (inner, ta),
                _ => (g, &[]),
            };
            if matches!(&g.kind, TermKind::Select(..) | TermKind::SelectIn(..) | TermKind::Path(_)) {
                let callee = self.conv_expr(cv, g);
                let tys: Vec<TyExprId> = ext_targs.iter().chain(targs.iter()).map(|a| self.conv_type_arg(cv, a)).collect();
                let tl = push_list(&mut cv.ast.ty_lists, &tys);
                let mut applied = cv.expr(Expr::TypeApply(callee, tl));
                for list in ext_lists {
                    let items: Vec<ExprId> = list.iter().map(|a| self.conv_arg(cv, a)).collect();
                    let il = push_list(&mut cv.ast.expr_lists, &items);
                    let using = !list.is_empty() && list.iter().all(|a| self.is_given_argument(cv, a));
                    applied = cv.expr(if using { Expr::UsingApply(applied, il) } else { Expr::Apply(applied, il) });
                }
                let using = !args.is_empty() && args.iter().all(|a| self.is_given_argument(cv, a));
                return cv.expr(if using { Expr::UsingApply(applied, l) } else { Expr::Apply(applied, l) });
            }
        }
        if let TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) = &head.kind {
            let simple = cv.tasty.simple(cv.tasty.source_name(*n)).map(str::to_string);
            match (&q.kind, simple.as_deref()) {
                // `new C[T](args)`: the constructor call as `NEW` under `<init>`.
                (TermKind::New(tpt), Some("<init>")) => {
                    if let TType::LocalType(addr, _) = tpt {
                        if let Some(&d) = cv.anon_defs.get(addr) {
                            return cv.expr(Expr::NewAnon(d));
                        }
                    }
                    if items.is_empty() && self.is_js_array_class(cv, tpt) {
                        return self.empty_array(cv, targs);
                    }
                    let ty = self.conv_new_type(cv, tpt, targs, inferred);
                    let new = cv.expr(Expr::New(ty, l));
                    if let Some(outer) = self.new_outer(cv, tpt) {
                        cv.ast.new_outers.insert(new, outer);
                    }
                    return new;
                }
                // `this.<init>(args)` in a secondary constructor.
                (TermKind::Path(TType::This(_)) | TermKind::QualThis(_), Some("<init>")) => {
                    let this = cv.expr(Expr::This);
                    return cv.expr(Expr::Apply(this, l));
                }
                // `x.synchronized(body)` is the body on JavaScript.
                (_, Some("synchronized")) if items.len() == 1 => return items[0],
                // `TupleN.apply(args)` is the tuple.
                (TermKind::Path(TType::TermRef(prefix, tn)), Some("apply")) if self.is_scala_tuple_path(cv, prefix, *tn) => {
                    return cv.expr(Expr::Tuple(l));
                }
                _ => {}
            }
        }
        // `X.apply(args)` on a stable path is `X(args)`, which the typer reads as the
        // constructor, companion `apply` or function call it is. The type arguments of a
        // value's own `apply` (`P.parallel.apply[B](x)` on a `FunctionK`) stay on `apply`.
        if let TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) = &head.kind {
            let stable = matches!(&q.kind, TermKind::Path(_) | TermKind::QualThis(_) | TermKind::Select(..) | TermKind::SelectIn(..));
            if stable && cv.tasty.simple(cv.tasty.source_name(*n)) == Some("apply") {
                let mut callee = self.conv_expr(cv, q);
                if let TermKind::TypeApply(_, targs, inferred) = &f.kind {
                    let value = match &q.kind {
                        TermKind::Path(p) => {
                            let mut cx = super::MapCx::new(cv.file);
                            self.path_term(&mut cx, p).map_or(false, |s| !matches!(self.syms.sym(s).kind, SymKind::Object(_)))
                        }
                        TermKind::Select(..) | TermKind::SelectIn(..) => true,
                        _ => false,
                    };
                    if value {
                        callee = cv.expr(Expr::Select(callee, names::APPLY));
                    }
                    let items: Vec<TyExprId> = targs.iter().map(|a| self.conv_type_arg(cv, a)).collect();
                    let tl = push_list(&mut cv.ast.ty_lists, &items);
                    if *inferred {
                        cv.ast.inferred_type_lists.push(tl.start);
                    }
                    callee = cv.expr(Expr::TypeApply(callee, tl));
                }
                let using = !args.is_empty() && args.iter().all(|a| self.is_given_argument(cv, a)) && !self.fills_plain_clause(cv, f);
                return cv.expr(if using { Expr::UsingApply(callee, l) } else { Expr::Apply(callee, l) });
            }
        }
        // `q.x_=(v)`, a setter called by its name as Scala 3.3 pickles `q.x = v`, is the
        // assignment, which the typer reads as the var's or as a call of the setter method; a
        // call of a written overload of the name stays the call.
        if let (TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..), [_]) = (&head.kind, args) {
            let text = cv.tasty.name(cv.tasty.source_name(*n));
            let field = text.strip_suffix("_=").filter(|f| f.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_'));
            if let (Some(field), true, true, false) = (field, targs.is_empty(), std::ptr::eq(head, f), self.calls_written_setter(cv, head)) {
                let field = self.interner.intern(field);
                let qe = self.conv_expr(cv, q);
                let lhs = cv.expr(Expr::Select(qe, field));
                return cv.expr(Expr::Assign(lhs, items[0]));
            }
        }
        // `a op b` for an operator, which the typer types as an infix expression.
        if let (TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..), [arg]) = (&head.kind, args) {
            let name = self.select_name(cv, *n);
            // A right-associative operator's receiver is the pickled one: an infix expression
            // would have the typer swap the operands again.
            let symbolic = (self.name_ref(name).chars().next().map_or(false, |c| !c.is_alphanumeric() && c != '_') && !self.name_ref(name).ends_with(':'))
                || matches!(name, names::EQ | names::NE);
            // A tuple built for the argument stays one argument: infix would spread it, but for
            // `==` and `!=`, whose right operand the typer takes whole.
            let tuple_arg = items.first().map_or(false, |&e| matches!(cv.ast.expr(e), Expr::Tuple(_))) && !matches!(name, names::EQEQ | names::NEQ);
            if symbolic && !matches!(&arg.kind, TermKind::NamedArg(..)) && std::ptr::eq(head, f) && !tuple_arg {
                let l = self.conv_expr(cv, q);
                let r = items[0];
                // The operator's selection, whose point is the operator's: what a diagnostic of
                // it names, as scalac's does.
                let e = cv.at(head.at, |cv| cv.expr(Expr::Infix(l, name, r)));
                self.keep_declaration(cv, head, e);
                return e;
            }
        }
        // `t.unary_!(ev)`: a prefix operator with a clause of its own is the method applied, not
        // the operator (zio-test's `TestTrace.unary_!` takes `A <:< Boolean`).
        let applied_prefix = match &f.kind {
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) if !args.is_empty() => {
                let name = self.select_name(cv, *n);
                self.name_ref(name).starts_with("unary_").then_some((q, name))
            }
            _ => None,
        };
        let fe = match applied_prefix {
            Some((q, name)) => {
                let qe = self.conv_expr(cv, q);
                cv.expr(Expr::Select(qe, name))
            }
            None => self.conv_expr(cv, f),
        };
        let using = !args.is_empty() && args.iter().all(|a| self.is_given_argument(cv, a)) && !self.fills_plain_clause(cv, f);
        cv.expr(if using { Expr::UsingApply(fe, l) } else { Expr::Apply(fe, l) })
    }

    /// The prefix of `new p.C` when it is a value path and `C` a class nested in a class:
    /// the enclosing instance the new one takes (`ExprsPlatform.this.Type` of `new Type.Cache`).
    fn new_outer(&mut self, cv: &mut Conv, tpt: &TType) -> Option<ExprId> {
        let class_ref = match tpt {
            TType::Applied(head, _) => &**head,
            other => other,
        };
        let prefix = match class_ref {
            TType::TypeRef(prefix, _) if matches!(&**prefix, TType::TermRef(..) | TType::LocalTerm(..)) => prefix,
            // A class of the same file under a path, or under a `this`: `new Node(e)` inside a
            // `Node` that extends its outer class (scala-library's `ListSet`) makes the node's own
            // member class, and `new t.Node(e)` the member class of `t`.
            TType::LocalType(_, Some(prefix)) if matches!(&**prefix, TType::This(_) | TType::TermRef(..) | TType::LocalTerm(..)) => prefix,
            _ => return None,
        };
        if self.is_package_path(cv, prefix) {
            return None;
        }
        let mut cx = super::MapCx::new(cv.file);
        let ctor = self.map_type_ctor(&mut cx, class_ref);
        // A class under a path of a local (`t.Node`) is known once the body is typed, where the
        // typer passes the prefix only to a class that takes an outer instance.
        if let Type::Class(c, _) | Type::Ctor(c) = self.types.get(ctor) {
            self.outer_class(c)?;
        }
        Some(self.conv_path(cv, prefix))
    }

    /// `scala.scalajs.js.Array`, the JS array, which teq's `Array` is.
    fn is_js_array_class(&self, cv: &Conv, tpt: &TType) -> bool {
        let class_ref = match tpt {
            TType::Applied(head, _) => &**head,
            other => other,
        };
        matches!(class_ref, TType::TypeRef(prefix, n) if cv.tasty.simple(cv.tasty.source_name(*n)) == Some("Array") && super::is_scalajs_path(&cv.tasty, prefix))
    }

    /// `new js.Array[T]()`, Scala.js's empty JS array: `Array.empty[T]`.
    fn empty_array(&mut self, cv: &mut Conv, targs: &[TType]) -> ExprId {
        let mut path = cv.expr(Expr::Ident(names::ROOT));
        for seg in ["scala", "Array", "empty"] {
            let n = self.interner.intern(seg);
            path = cv.expr(Expr::Select(path, n));
        }
        if targs.is_empty() {
            return path;
        }
        let tys: Vec<TyExprId> = targs.iter().map(|a| self.conv_type_arg(cv, a)).collect();
        let tl = push_list(&mut cv.ast.ty_lists, &tys);
        cv.expr(Expr::TypeApply(path, tl))
    }

    /// The type a constructor call `new C[T](args)` constructs, when `t` is one.
    fn constructed_new<'t>(&self, cv: &Conv, t: &'t Term) -> Option<&'t TType> {
        let TermKind::Apply(f, _) = &t.kind else { return None };
        let head = match &f.kind {
            TermKind::TypeApply(inner, ..) => &**inner,
            _ => f,
        };
        match &head.kind {
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) if cv.tasty.simple(cv.tasty.source_name(*n)) == Some("<init>") => match &q.kind {
                TermKind::New(tpt) => Some(tpt),
                _ => None,
            },
            _ => None,
        }
    }

    /// `scala.runtime.TupleXXL`, the companion whose `unapplySeq` a tuple pattern past 22
    /// elements is in a pickle.
    fn is_tuple_xxl_path(&self, cv: &Conv, prefix: &TType, n: u32) -> bool {
        let TType::Package(p) = prefix else { return false };
        cv.tasty.name(*p) == "scala.runtime" && cv.tasty.simple(cv.tasty.source_name(n)) == Some("TupleXXL")
    }

    /// Whether a sub-pattern of `TupleXXL(..)` is a wildcard, or a binder of one, declared with a
    /// type other than `Any`: an element's, which the extractor's `Seq[Any]` does not give.
    fn declares_element(&self, cv: &Conv, p: &Term) -> bool {
        let wild = |t: &Term| matches!(&t.kind, TermKind::Ident(w, _) if cv.tasty.simple(*w) == Some("_"));
        let ty = match &p.kind {
            TermKind::Bind { ty, body, .. } if wild(body) => &**ty,
            TermKind::Ident(_, ty) if wild(p) => ty,
            _ => return false,
        };
        !matches!(ty, TType::TypeRef(_, n) if cv.tasty.simple(*n) == Some("Any"))
    }

    fn is_scala_tuple_path(&self, cv: &Conv, prefix: &TType, n: u32) -> bool {
        let TType::Package(p) = prefix else { return false };
        if cv.tasty.simple(*p) != Some("scala") {
            return false;
        }
        cv.tasty.simple(cv.tasty.source_name(n)).and_then(|s| s.strip_prefix("Tuple")).map_or(false, |d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
    }

    /// Whether an argument is a given instance, which scalac passed to a using clause.
    /// `g[ext_targs ++ targs]` applied to the extension's clauses in order, each a using
    /// application where all its arguments are given ones.
    fn conv_extension_call(&mut self, cv: &mut Conv, g: &Term, ext_targs: &[TType], targs: &[TType], clauses: &[&[Term]]) -> ExprId {
        let callee = self.conv_expr(cv, g);
        let tys: Vec<TyExprId> = ext_targs.iter().chain(targs.iter()).map(|a| self.conv_type_arg(cv, a)).collect();
        let tl = push_list(&mut cv.ast.ty_lists, &tys);
        let mut applied = cv.expr(Expr::TypeApply(callee, tl));
        for clause in clauses {
            let items: Vec<ExprId> = clause.iter().map(|a| self.conv_arg(cv, a)).collect();
            let rl = push_list(&mut cv.ast.expr_lists, &items);
            let using = !clause.is_empty() && clause.iter().all(|a| self.is_given_argument(cv, a));
            applied = cv.expr(if using { Expr::UsingApply(applied, rl) } else { Expr::Apply(applied, rl) });
        }
        applied
    }

    fn is_given_argument(&mut self, cv: &mut Conv, a: &Term) -> bool {
        if self.is_type_of_call(cv, a) {
            return true;
        }
        match &a.kind {
            TermKind::Path(TType::LocalTerm(addr, _)) => cv.local_givens.contains_key(addr) || cv.quotes_params.contains_key(addr) || self.term_sym(cv, *addr).map_or(false, |s| self.syms.is_given(s)),
            TermKind::Path(TType::TermRef(prefix, n)) => {
                let name = self.conv_name(cv, *n);
                match self.static_path_scope(cv, prefix) {
                    Some(r) => match r {
                        PathScope::Pkg(p) => self.pkg_term(p, name).and_then(|r| r.sym()).map_or(false, |s| self.syms.is_given(s)),
                        PathScope::Module(c) => self.module_term(c, name).and_then(|r| r.sym()).map_or(false, |s| self.syms.is_given(s)),
                    },
                    None => false,
                }
            }
            TermKind::Apply(f, args) => self.is_given_argument(cv, f) && (self.next_clause_is_using(cv, f) || args.iter().all(|a| self.is_given_argument(cv, a))),
            TermKind::TypeApply(f, ..) => self.is_given_argument(cv, f),
            _ => false,
        }
    }

    /// Whether the clause a call `f(..)` fills is a using clause of the method on a static path
    /// that `f` applies: scalac passes such a clause what it synthesized (`NotGiven.value`), which
    /// is no given of its own.
    fn next_clause_is_using(&mut self, cv: &mut Conv, f: &Term) -> bool {
        let mut applied = 0;
        let mut head = f;
        loop {
            match &head.kind {
                TermKind::Apply(g, _) => {
                    applied += 1;
                    head = g;
                }
                TermKind::TypeApply(g, ..) => head = g,
                _ => break,
            }
        }
        let sym = match &head.kind {
            TermKind::Path(TType::TermRef(prefix, n)) => {
                let name = self.conv_name(cv, *n);
                match self.static_path_scope(cv, prefix) {
                    Some(PathScope::Pkg(p)) => self.pkg_term(p, name).and_then(|r| r.sym()),
                    Some(PathScope::Module(c)) => self.module_term(c, name).and_then(|r| r.sym()),
                    None => None,
                }
            }
            TermKind::Path(TType::LocalTerm(addr, _)) => self.term_sym(cv, *addr),
            _ => None,
        };
        let Some(s) = sym else { return false };
        if matches!(self.syms.sym(s).kind, SymKind::Overloaded(_)) {
            return false;
        }
        self.sig_of(s).clauses.get(applied).map_or(false, |c| c.is_using || c.is_implicit)
    }

    /// Whether the clause a call `f(..)` fills is a using one, for each method `f` may name that
    /// has a clause there: a member its selection names in the class that declares it, or the
    /// method on a static or local path. Given arguments of an ordinary clause are written for its
    /// parameters (`xs.mkString(sep)` of a using parameter `sep`), no using application, which
    /// would apply `mkString()`'s result.
    fn filled_clauses(&mut self, cv: &mut Conv, f: &Term) -> Vec<bool> {
        let mut applied = 0;
        let mut head = f;
        loop {
            match &head.kind {
                TermKind::Apply(g, _) => {
                    applied += 1;
                    head = g;
                }
                TermKind::TypeApply(g, ..) => head = g,
                _ => break,
            }
        }
        let entries: Vec<SymId> = match &head.kind {
            TermKind::SelectIn(_, n, owner, _) => {
                let Some(c) = self.this_class_of(cv, owner) else { return Vec::new() };
                let name = self.select_name(cv, *n);
                self.complete_class(c);
                self.syms.class(c).base_types.iter().filter_map(|&(b, _)| self.syms.class(b).members.get(&name).copied()).collect()
            }
            TermKind::Path(TType::TermRef(prefix, n)) => {
                let name = self.conv_name(cv, *n);
                let entry = match self.static_path_scope(cv, prefix) {
                    Some(PathScope::Pkg(p)) => self.pkg_term(p, name).and_then(|r| r.sym()),
                    Some(PathScope::Module(c)) => self.module_term(c, name).and_then(|r| r.sym()),
                    None => None,
                };
                entry.into_iter().collect()
            }
            TermKind::Path(TType::LocalTerm(addr, _)) => self.term_sym(cv, *addr).into_iter().collect(),
            _ => return Vec::new(),
        };
        let methods: Vec<SymId> = entries.iter().flat_map(|&e| self.syms.alternatives(e).map_or_else(|| vec![e], |a| a.to_vec())).collect();
        // An alternative of no clause there (`mkString` beside `mkString(sep)`) takes none.
        methods.into_iter().filter_map(|m| self.sig_of(m).clauses.get(applied).map(|c| c.is_using || c.is_implicit)).collect()
    }

    /// Whether the clause a call `f(..)` fills is an ordinary one of every method `f` may name.
    fn fills_plain_clause(&mut self, cv: &mut Conv, f: &Term) -> bool {
        let clauses = self.filled_clauses(cv, f);
        !clauses.is_empty() && clauses.into_iter().all(|using| !using)
    }

    /// Whether the clause a call `f(..)` fills is a using clause of every method `f` may name.
    fn fills_using_clause(&mut self, cv: &mut Conv, f: &Term) -> bool {
        let clauses = self.filled_clauses(cv, f);
        !clauses.is_empty() && clauses.into_iter().all(|using| using)
    }

    fn static_path_scope(&mut self, cv: &mut Conv, t: &TType) -> Option<PathScope> {
        match t {
            TType::Package(p) => {
                let text = cv.tasty.name(*p);
                let mut pkg = ROOT_PKG;
                for seg in text.split('.').filter(|s| !s.is_empty()) {
                    let n = self.interner.intern(seg);
                    pkg = self.syms.pkg(pkg).entries.get(&n).and_then(|e| e.pkg)?;
                }
                Some(PathScope::Pkg(pkg))
            }
            TType::TermRef(prefix, n) => {
                let name = self.conv_name(cv, *n);
                let r = match self.static_path_scope(cv, prefix)? {
                    PathScope::Pkg(p) => self.pkg_term(p, name)?,
                    PathScope::Module(c) => self.module_term(c, name)?,
                };
                match self.syms.sym(r.sym()?).kind {
                    SymKind::Object(c) => Some(PathScope::Module(c)),
                    _ => None,
                }
            }
            TType::LocalTerm(addr, _) => match self.term_sym(cv, *addr) {
                Some(s) => match self.syms.sym(s).kind {
                    SymKind::Object(c) => Some(PathScope::Module(c)),
                    _ => None,
                },
                None => None,
            },
            TType::This(inner) => match &**inner {
                TType::TypeRef(prefix, n) => {
                    let name = self.conv_name(cv, *n);
                    match self.static_path_scope(cv, prefix)? {
                        PathScope::Pkg(p) => match self.pkg_term(p, name)?.sym().map(|s| self.syms.sym(s).kind) {
                            Some(SymKind::Object(c)) => Some(PathScope::Module(c)),
                            _ => None,
                        },
                        PathScope::Module(c) => match self.module_term(c, name)?.sym().map(|s| self.syms.sym(s).kind) {
                            Some(SymKind::Object(m)) => Some(PathScope::Module(m)),
                            _ => None,
                        },
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// A block: the local methods that closures of the body are over stay out of it for the
    /// closure to take; a local class becomes the definition its `new` instantiates.
    fn conv_block(&mut self, cv: &mut Conv, stats: &[Stat], expr: &Term) -> ExprId {
        if !cv.product {
            return self.conv_block_scoped(cv, stats, expr);
        }
        let mark = cv.scope.len();
        let e = self.conv_block_scoped(cv, stats, expr);
        cv.scope.truncate(mark);
        e
    }

    fn conv_block_scoped(&mut self, cv: &mut Conv, stats: &[Stat], expr: &Term) -> ExprId {
        if let Some(lambda) = self.conv_poly_function_class(cv, stats, expr) {
            return lambda;
        }
        if self.converts_product(cv) && !cv.scope.is_empty() {
            for (i, s) in stats.iter().enumerate() {
                let Stat::Val(sig, rhs) = s else { continue };
                let mut within: Vec<&Term> = rhs.iter().collect();
                within.extend(stats[i + 1..].iter().filter_map(|s| match s {
                    Stat::Val(_, Some(t)) | Stat::Def(_, Some(t)) | Stat::Expr(t) => Some(t),
                    _ => None,
                }));
                within.push(expr);
                let types: Vec<&TType> = stats[i..].iter().filter_map(|s| match s {
                    Stat::Val(sig, _) | Stat::Def(sig, _) => Some(&sig.ret),
                    _ => None,
                }).collect();
                self.note_shadowing(cv, &[(sig.addr, sig.name)], &within, &types);
            }
        }
        let mut stmts = Vec::with_capacity(stats.len() + 1);
        // A local method, lazy val, class, object or type may be named before its definition.
        for s in stats {
            match s {
                Stat::Def(sig, _) => {
                    let name = self.conv_name(cv, sig.name);
                    cv.locals.insert(sig.addr, name);
                }
                Stat::Val(sig, _) if sig.mods.flags.has(tags::OBJECT) || sig.mods.flags.has(tags::LAZY) => {
                    let name = self.conv_name(cv, sig.name);
                    cv.locals.insert(sig.addr, name);
                }
                Stat::Type(sig) => {
                    let name = self.conv_name(cv, sig.name);
                    cv.local_types.insert(sig.addr, name);
                }
                Stat::Class(cd) if cv.tasty.name(cv.tasty.source_name(cd.name)) != "$anon" => {
                    let name = self.lname(&cv.tasty.clone(), cv.tasty.source_name(cd.name));
                    if cd.mods.flags.has(tags::OBJECT) {
                        cv.local_modules.insert(cd.addr, name);
                    } else {
                        cv.local_types.insert(cd.addr, name);
                    }
                }
                _ => {}
            }
        }
        self.hold_lambda_defs(cv, stats);
        for s in stats {
            match s {
                Stat::Def(sig, Some(_)) if cv.lambda_targets.contains(&sig.addr) => {}
                // A local object is its class's definition, which holds the instance.
                Stat::Val(sig, _) if sig.mods.flags.has(tags::OBJECT) => {}
                Stat::Class(cd) => {
                    let mark = cv.self_vals.len();
                    let d = self.conv_local_class(cv, cd);
                    // The enclosing local class's `this`, which the class just converted
                    // names, as a val of this block for it to capture.
                    let here = cv.local_classes.last().copied();
                    let mut requested: Vec<Name> = Vec::new();
                    cv.self_vals.retain(|&(addr, name)| {
                        if Some(addr) == here {
                            requested.push(name);
                            false
                        } else {
                            true
                        }
                    });
                    let _ = mark;
                    for name in requested {
                        let this = cv.expr(Expr::This);
                        let d = cv.ast.add_def(Def { name, span: cv.span, mods: 0, annots: Vec::new(), kind: DefKind::Val { pat: None, ty: None, rhs: Some(this) } });
                        stmts.push(Stmt::Def(d));
                    }
                    if let Some(d) = d {
                        stmts.push(Stmt::Def(d));
                    }
                }
                other => self.conv_stat(cv, other, &mut stmts),
            }
        }
        let e = self.conv_expr(cv, expr);
        if stmts.is_empty() {
            return e;
        }
        stmts.push(Stmt::Expr(e));
        let l = push_list(&mut cv.ast.stmts, &stmts);
        cv.expr(Expr::Block(l))
    }

    /// A polymorphic function literal as Scala 3.3 pickles it: a block of one anonymous class
    /// extending `scala.PolyFunction` with a polymorphic `apply`, instantiated and ascribed
    /// the refinement type; the literal `[t] => (params) => body`.
    fn conv_poly_function_class(&mut self, cv: &mut Conv, stats: &[Stat], expr: &Term) -> Option<ExprId> {
        let [Stat::Class(cd)] = stats else { return None };
        let extends_poly = cd.template.parents.iter().any(|p| match &p.kind {
            TermKind::Path(t) | TermKind::New(t) => self.is_poly_function_parent(cv, t),
            TermKind::Apply(f, _) => matches!(&f.kind, TermKind::Select(q, _) | TermKind::SelectIn(q, _, ..) if matches!(&q.kind, TermKind::New(t) if self.is_poly_function_parent(cv, t))),
            _ => false,
        });
        if !extends_poly {
            return None;
        }
        let instantiates = |t: &Term| match &t.kind {
            TermKind::Apply(f, args) if args.is_empty() => matches!(&f.kind, TermKind::Select(q, _) | TermKind::SelectIn(q, _, ..) if matches!(&q.kind, TermKind::New(TType::LocalType(addr, _)) if *addr == cd.addr)),
            _ => false,
        };
        match &expr.kind {
            TermKind::Typed(inner, ..) if instantiates(inner) => {}
            _ if instantiates(expr) => {}
            _ => return None,
        }
        let mut apply = None;
        for s in &cd.template.stats {
            match s {
                Stat::Def(sig, Some(rhs)) if cv.tasty.simple(cv.tasty.source_name(sig.name)) == Some("apply") && apply.is_none() => apply = Some((sig, rhs)),
                Stat::Def(..) | Stat::Val(..) | Stat::Class(_) => return None,
                _ => {}
            }
        }
        let (sig, rhs) = apply?;
        if !sig.clauses.iter().any(|c| matches!(c, Clause::Types(_))) {
            return None;
        }
        Some(self.conv_lambda(cv, sig, rhs, None))
    }

    /// The methods of the closures among `stats`, held for the closures to take: the statement
    /// that takes a closure may precede its method.
    fn hold_lambda_defs(&mut self, cv: &mut Conv, stats: &[Stat]) {
        for s in stats {
            if let Stat::Def(sig, Some(rhs)) = s {
                if cv.lambda_targets.contains(&sig.addr) {
                    cv.lambda_defs.insert(sig.addr, (sig.clone(), rhs.clone()));
                }
            }
        }
    }

    /// A class defined in a block: `new T { ... }`, whose definition the typer enters when it
    /// meets the `new`, or a named local class, which is a definition of the block; the members
    /// of either are referred to by name.
    fn conv_local_class(&mut self, cv: &mut Conv, cd: &TClassDef) -> Option<DefId> {
        let tasty = cv.tasty.clone();
        let mut tparams = Vec::with_capacity(cd.template.tparams.len());
        for p in &cd.template.tparams {
            let n = self.conv_name(cv, p.name);
            cv.local_types.insert(p.addr, n);
            let arity = super::types::hk_arity(&p.info);
            let (lower, upper) = match &p.info {
                TType::Bounds(lo, hi) if arity == 0 => (Some(self.conv_type(cv, lo)), Some(self.conv_type(cv, hi))),
                _ => (None, None),
            };
            let variance = if p.flags.has(tags::COVARIANT) { 1 } else if p.flags.has(tags::CONTRAVARIANT) { -1 } else { 0 };
            tparams.push(TypeParam { name: n, span: cv.span, variance, arity, hk_variances: Vec::new(), upper, lower, context_bounds: Vec::new(), evidence_names: Vec::new(), annots: Vec::new() });
        }
        let tasty_name = tasty.name(tasty.source_name(cd.name));
        let named = tasty_name != "$anon";
        if !named && !cd.template.params.is_empty() {
            cv.refuse("an anonymous class with parameters");
            return None;
        }
        let name = self.lname(&tasty, tasty.source_name(cd.name));
        let flags = cd.mods.flags;
        let module = flags.has(tags::OBJECT);
        // The companion scalac adds to a case class holds only what the typer derives again.
        if named && module && flags.has(tags::SYNTHETIC) {
            return None;
        }
        if named && !module {
            cv.local_types.insert(cd.addr, name);
        }
        let derived = flags.has(tags::CASE) || module;
        let mut params = Vec::with_capacity(cd.template.params.len());
        for p in &cd.template.params {
            let pname = self.conv_name(cv, p.name);
            cv.locals.insert(p.addr, pname);
            let ty = self.conv_param_type(cv, &p.ty);
            let field = if p.flags.has(tags::LOCAL) { 0 } else { mods::FIELD | flag_mods(p.flags) & mods::MUTABLE };
            params.push(Param { name: pname, span: cv.span, ty, default: None, mods: field, annots: ListRef::EMPTY });
        }
        let clauses = if cd.template.params.is_empty() { Vec::new() } else { vec![ParamClause { params, is_using: false, is_implicit: false }] };
        for s in &cd.template.stats {
            match s {
                Stat::Val(sig, _) | Stat::Def(sig, _) => {
                    let name = self.lname(&tasty, tasty.source_name(sig.name));
                    cv.locals.insert(sig.addr, name);
                    if let (Stat::Def(_, Some(rhs)), true) = (s, cv.lambda_targets.contains(&sig.addr)) {
                        cv.lambda_defs.insert(sig.addr, (sig.clone(), rhs.clone()));
                    }
                }
                // A reference to the class's own type member points at its definition.
                Stat::Type(sig) => {
                    let name = self.lname(&tasty, tasty.source_name(sig.name));
                    cv.local_types.insert(sig.addr, name);
                }
                // A class of the local class (a case class inside an anonymous one, cats'
                // `Deferred` of `catsSddDeferForFunction0`) may be named before its definition.
                Stat::Class(inner) => {
                    let name = self.lname(&tasty, tasty.source_name(inner.name));
                    if inner.mods.flags.has(tags::OBJECT) {
                        cv.local_modules.insert(inner.addr, name);
                    } else {
                        cv.local_types.insert(inner.addr, name);
                    }
                }
                _ => {}
            }
        }
        cv.local_classes.push(cd.addr);
        // A product's local class's parents' calls see its constructor's parameters as plain
        // names in scope, as a class's do (`conv_class_def`).
        let scope_mark = cv.scope.len();
        let outer_params = cv.parent_params.take();
        if self.converts_product(cv) {
            let names: Vec<Name> = cd.template.params.iter().map(|p| cv.locals[&p.addr]).collect();
            cv.scope.extend(cd.template.params.iter().map(|p| (cv.locals[&p.addr], p.addr)));
            cv.parent_params = Some((cd.addr, names));
        }
        let mut parents = self.conv_template_parents(cv, &cd.template.parents, !named);
        // A product's local case class is read as the program's own: its pickle extends
        // `Product` and `Serializable` as scalac's `Desugar` makes it, which teq's typer gives a
        // case class by its rules, as the loader reads a top-level one.
        if self.converts_product(cv) && named && !module && flags.has(tags::CASE) {
            let product = self.b.product;
            parents.retain(|p| match cv.ast.tys[p.ty.idx()] {
                TyExpr::Resolved(t) => match self.types.get(t) {
                    Type::Class(pc, _) => Some(pc) != product && !(self.name_str(self.syms.class(pc).name) == "Serializable" && matches!(self.syms.class(pc).owner, Owner::Package(k) if self.pkg_description(k) == "java.io")),
                    _ => true,
                },
                _ => true,
            });
        }
        cv.parent_params = outer_params;
        cv.scope.truncate(scope_mark);
        // Inside the local class `C.this` of an enclosing class is an outer `this`.
        cv.this_chain.push(LOCAL_CLASS);
        let mut body = Vec::with_capacity(cd.template.stats.len());
        // A var's generated setter of a product's local class, which teq's typer makes for the var
        // as the whole build's does (`Loaded::product_synthetics` for a product's own classes); a
        // written overload of the setter's name is the source's.
        let setters: Vec<String> = match self.converts_product(cv) {
            true => cd.template.stats.iter().filter_map(|s| match s {
                Stat::Val(sig, _) if sig.mods.flags.has(tags::MUTABLE) => tasty.simple(tasty.source_name(sig.name)).map(|n| format!("{}_=", n)),
                _ => None,
            }).collect(),
            false => Vec::new(),
        };
        for s in &cd.template.stats {
            let Stat::Def(sig, _) = s else { continue };
            let Some(name) = tasty.simple(tasty.source_name(sig.name)).filter(|n| setters.iter().any(|s| s == n)) else { continue };
            if sig.mods.flags.has(tags::FIELDACCESSOR) {
                continue;
            }
            let param = match sig.clauses.as_slice() {
                [Clause::Terms(ps)] if ps.len() == 1 => self.qualified_type_name(cv, &ps[0].ty),
                _ => None,
            };
            if let Some(param) = param {
                cv.written_setters.insert((cd.addr, name.to_string(), param), ());
            }
        }
        for s in &cd.template.stats {
            match s {
                Stat::Val(sig, _) | Stat::Def(sig, _) if named && derived && sig.mods.flags.has(tags::SYNTHETIC) => {}
                Stat::Def(sig, _) if !setters.is_empty() && sig.mods.flags.has(tags::FIELDACCESSOR) && tasty.simple(tasty.source_name(sig.name)).map_or(false, |n| setters.iter().any(|s| s == n)) => {}
                Stat::Type(sig) if named && derived && sig.mods.flags.has(tags::SYNTHETIC) => {}
                // A nested object is its class's definition, which holds the instance.
                Stat::Val(sig, _) if sig.mods.flags.has(tags::OBJECT) => {}
                Stat::Class(inner) => {
                    if let Some(d) = self.conv_local_class(cv, inner) {
                        body.push(Stmt::Def(d));
                    }
                }
                Stat::Val(sig, rhs) => {
                    let name = self.conv_name(cv, sig.name);
                    let ty = self.conv_type(cv, &sig.ret);
                    let init = rhs.as_ref().map(|r| self.conv_expr(cv, r));
                    let m = flag_mods(sig.mods.flags) & (mods::LAZY | mods::MUTABLE | mods::OVERRIDE | mods::FINAL | mods::IMPLICIT | mods::GIVEN | qualified_access(&sig.mods));
                    let d = cv.ast.add_def(Def { name, span: cv.span, mods: m, annots: Vec::new(), kind: DefKind::Val { pat: None, ty: Some(ty), rhs: init } });
                    cv.def_at.push((d, sig.addr));
                    body.push(Stmt::Def(d));
                }
                Stat::Def(sig, Some(_)) if cv.lambda_targets.contains(&sig.addr) => {}
                Stat::Def(sig, rhs) => {
                    let d = self.conv_def(cv, sig, rhs.as_ref(), true);
                    body.push(Stmt::Def(d));
                }
                Stat::Expr(e) => {
                    let x = self.conv_expr(cv, e);
                    body.push(Stmt::Expr(x));
                }
                Stat::Type(sig) => {
                    if let Some(d) = self.conv_type_alias(cv, sig) {
                        body.push(Stmt::Def(d));
                    }
                }
                _ => {}
            }
        }
        cv.this_chain.pop();
        cv.local_classes.pop();
        let (kind, class_mods) = if !named {
            (ast::ClassKind::Class, mods::FINAL)
        } else if module {
            (ast::ClassKind::Object, flag_mods(flags) & mods::CASE)
        } else {
            let kind = if flags.has(tags::TRAIT) { ast::ClassKind::Trait } else { ast::ClassKind::Class };
            (kind, flag_mods(flags) & (mods::CASE | mods::SEALED | mods::ABSTRACT | mods::FINAL))
        };
        let cls = ast::ClassDef { kind, tparams, clauses, parents, body, exports: ListRef::EMPTY, self_type: None, self_alias: crate::names::EMPTY };
        let d = cv.ast.add_def(Def { name, span: cv.span, mods: class_mods, annots: Vec::new(), kind: DefKind::Class(Box::new(cls)) });
        cv.def_at.push((d, cd.addr));
        if named {
            return Some(d);
        }
        cv.anon_defs.insert(cd.addr, d);
        None
    }

    /// `type TypeClassType = Show[A]`: the alias refines the instance's type; a parameterised
    /// one (`type Typeclass[T] = Schema[T]`) is a lambda in TASTy, whose parameters become the
    /// alias's.
    fn conv_type_alias(&mut self, cv: &mut Conv, sig: &crate::tasty::tree::TypeDefSig) -> Option<DefId> {
        // An abstract type member (`type P <: String` of a local object), with its bounds.
        if let TType::Bounds(lo, hi) = &sig.rhs {
            let name = self.conv_name(cv, sig.name);
            let lower = (!self.names_nothing(cv, lo)).then(|| self.conv_type(cv, lo));
            let upper = (!self.names_any(cv, hi)).then(|| self.conv_type(cv, hi));
            return Some(cv.ast.add_def(Def { name, span: cv.span, mods: 0, annots: Vec::new(), kind: DefKind::TypeAlias { tparams: Vec::new(), rhs: None, lower, upper } }));
        }
        {
                    let name = self.conv_name(cv, sig.name);
                    let (tparams, rhs) = match &sig.rhs {
                        TType::Lambda { kind: LambdaKind::Type, binder, params, result } => {
                            let names: Vec<Name> = params.iter().map(|p| self.conv_name(cv, p.name)).collect();
                            // The parameters are `TYPEPARAM`s of the alias, named in its
                            // right-hand side by their address.
                            for (p, &n) in params.iter().zip(&names) {
                                if p.addr != 0 {
                                    cv.local_types.insert(p.addr, n);
                                }
                            }
                            cv.poly_binders.push((*binder, names.clone()));
                            let rhs = self.conv_type(cv, result);
                            cv.poly_binders.pop();
                            let tparams = names
                                .iter()
                                .zip(params)
                                .map(|(&n, p)| TypeParam { name: n, span: cv.span, variance: 0, arity: super::types::hk_arity(&p.info), hk_variances: Vec::new(), upper: None, lower: None, context_bounds: Vec::new(), evidence_names: Vec::new(), annots: Vec::new() })
                                .collect();
                            (tparams, rhs)
                        }
                        other => (Vec::new(), self.conv_type(cv, other)),
                    };
                    Some(cv.ast.add_def(Def { name, span: cv.span, mods: 0, annots: Vec::new(), kind: DefKind::TypeAlias { tparams, rhs: Some(rhs), lower: None, upper: None } }))
        }
    }

    fn names_nothing(&mut self, cv: &mut Conv, t: &TType) -> bool {
        let mut mx = super::MapCx::new(cv.file);
        !self.mentions_local_type(cv, t) && self.map_type(&mut mx, t) == NOTHING
    }

    fn names_any(&mut self, cv: &mut Conv, t: &TType) -> bool {
        let mut mx = super::MapCx::new(cv.file);
        !self.mentions_local_type(cv, t) && self.map_type(&mut mx, t) == ANY
    }

    fn conv_lambda(&mut self, cv: &mut Conv, sig: &DefSig, rhs: &Term, sam: Option<&TType>) -> ExprId {
        if !cv.product {
            return self.conv_lambda_scoped(cv, sig, rhs, sam);
        }
        let mark = cv.scope.len();
        if self.converts_product(cv) {
            let params: Vec<(Addr, NameRef)> = sig.clauses.iter().flat_map(|c| match c {
                Clause::Terms(ps) => ps.iter().map(|p| (p.addr, p.name)).collect(),
                Clause::Types(_) => Vec::new(),
            }).collect();
            let types: Vec<&TType> = sig.clauses.iter().flat_map(|c| match c {
                Clause::Terms(ps) => ps.iter().map(|p| &p.ty).collect(),
                Clause::Types(ps) => ps.iter().map(|p| &p.info).collect::<Vec<_>>(),
            }).collect();
            self.note_shadowing(cv, &params, &[rhs], &types);
        }
        let e = self.conv_lambda_scoped(cv, sig, rhs, sam);
        cv.scope.truncate(mark);
        e
    }

    fn conv_lambda_scoped(&mut self, cv: &mut Conv, sig: &DefSig, rhs: &Term, sam: Option<&TType>) -> ExprId {
        let mut params = Vec::new();
        // The type parameters of a polymorphic function literal (`[t] => (x: t) => ...`), named
        // for the body.
        let mut tparams = Vec::new();
        for clause in &sig.clauses {
            if let Clause::Types(tps) = clause {
                for tp in tps {
                    let name = self.conv_name(cv, tp.name);
                    cv.local_types.insert(tp.addr, name);
                    tparams.push(name);
                }
            }
            let Clause::Terms(ps) = clause else { continue };
            // `(using x: A) => e`, a context function literal; `implicit x => e` is a plain
            // function whose parameter is a given in the body.
            let implicit = ps.iter().all(|p| p.flags.has(tags::GIVEN) || p.flags.has(tags::IMPLICIT)) && !ps.is_empty();
            let contextual = implicit && ps.iter().all(|p| p.flags.has(tags::GIVEN));
            for p in ps {
                let name = self.conv_name(cv, p.name);
                let name = self.unshadowed(cv, p.addr, name);
                cv.locals.insert(p.addr, name);
                cv.local_val_types.insert(p.addr, p.ty.clone());
                if implicit {
                    cv.local_givens.insert(p.addr, ());
                }
                let ty = self.conv_param_type(cv, &p.ty);
                if p.inferred {
                    cv.mark_inferred(ty);
                }
                params.push(LambdaParam { name, span: cv.span, ty: Some(ty), implicit, contextual });
            }
        }
        let body = self.conv_expr(cv, rhs);
        if cv.product {
            if let Some((name, source, offset)) = self.product_sam_class(cv, sig.addr) {
                cv.reader().sam_classes.insert(body, (name, source, offset));
            }
        }
        let l = push_list(&mut cv.ast.lambda_params, &params);
        let mut lambda = cv.expr(Expr::Lambda(l, body));
        if !tparams.is_empty() {
            let tps: Vec<TParam> = sig.clauses.iter().flat_map(|c| match c {
                Clause::Types(tps) => tps.clone(),
                Clause::Terms(_) => Vec::new(),
            }).collect();
            let bounds = self.conv_lambda_bounds(cv, &tps);
            let tl = push_list(&mut cv.ast.name_lists, &tparams);
            lambda = cv.expr(Expr::PolyLambda(tl, lambda));
            if let Some(bl) = bounds {
                cv.ast.poly_lambda_bounds.push((lambda, bl));
            }
        }
        match sam {
            Some(t) => {
                let ty = self.conv_type(cv, t);
                cv.expr(Expr::Typed(lambda, ty))
            }
            None => lambda,
        }
    }

    fn conv_stat(&mut self, cv: &mut Conv, s: &Stat, out: &mut Vec<Stmt>) {
        match s {
            Stat::Val(sig, rhs) => {
                let name = self.conv_name(cv, sig.name);
                let name = self.unshadowed(cv, sig.addr, name);
                cv.locals.insert(sig.addr, name);
                cv.local_val_types.insert(sig.addr, sig.ret.clone());
                // What is read as the wildcard's bound holds a value of the wildcard type: the
                // initialiser, and what a var is assigned, are cast to the declared type, which
                // erasure makes free.
                let (ty, cast) = match self.local_val_type(cv, &sig.ret) {
                    Some(declared) => {
                        let ty = self.conv_type(cv, &declared);
                        cv.cast_locals.insert(sig.addr, ty);
                        (ty, true)
                    }
                    None => {
                        let ty = self.conv_type(cv, &sig.ret);
                        if !sig.mods.flags.has(tags::MUTABLE) && !sig.mods.flags.has(tags::LAZY) && sig.ret_inferred {
                            cv.mark_inferred(ty);
                        }
                        (ty, false)
                    }
                };
                let init = rhs.as_ref().map(|r| {
                    let e = self.conv_expr(cv, r);
                    if cast { self.cast_to(cv, e, ty) } else { e }
                });
                // A local holding a Predef wrapper (`val $4: StringOps = augmentString(s)`) holds
                // the wrapped value itself, whose type the initialiser gives.
                let wrapped = rhs.as_ref().map_or(false, |r| matches!(&r.kind, TermKind::Apply(f, _) if matches!(self.predef_wrapper(cv, f), Some(PredefWrapper::Identity))));
                let ty = if wrapped { None } else { Some(ty) };
                if sig.mods.flags.has(tags::GIVEN) {
                    let ty = ty.unwrap_or_else(|| self.conv_type(cv, &sig.ret));
                    cv.local_givens.insert(sig.addr, ());
                    let given = GivenDef { tparams: Vec::new(), clauses: Vec::new(), ty, alias: init, body: Vec::new(), self_alias: crate::names::EMPTY };
                    let d = cv.ast.add_def(Def { name, span: cv.span, mods: mods::LAZY, annots: Vec::new(), kind: DefKind::Given(Box::new(given)) });
                    cv.def_at.push((d, sig.addr));
                    out.push(Stmt::Def(d));
                    return;
                }
                let mut m = 0;
                if sig.mods.flags.has(tags::LAZY) {
                    m |= mods::LAZY;
                }
                if sig.mods.flags.has(tags::MUTABLE) {
                    m |= mods::MUTABLE;
                }
                let d = cv.ast.add_def(Def { name, span: cv.span, mods: m, annots: Vec::new(), kind: DefKind::Val { pat: None, ty, rhs: init } });
                cv.def_at.push((d, sig.addr));
                out.push(Stmt::Def(d));
            }
            Stat::Def(sig, rhs) => {
                let d = self.conv_def(cv, sig, rhs.as_ref(), false);
                out.push(Stmt::Def(d));
            }
            Stat::Class(_) => {
                let e = cv.refuse("a local class");
                out.push(Stmt::Expr(e));
            }
            Stat::Type(sig) => {
                if let Some(d) = self.conv_type_alias(cv, sig) {
                    out.push(Stmt::Def(d));
                }
            }
            Stat::Import { path, selectors } => {
                if let Some(s) = self.conv_import(cv, path, selectors, true) {
                    out.push(s);
                }
            }
            Stat::Expr(e) => {
                let x = self.conv_expr(cv, e);
                out.push(Stmt::Expr(x));
            }
        }
    }

    /// The name a binder of the body is converted under: its own, unless another binder of the
    /// body has it (`def of(value: Expr[value.Underlying])` inside a lambda over `value`, whose
    /// `value.Underlying` names the lambda's), which the names of the source would confuse. In a
    /// product's body a binder is renamed only where it shadows one in scope that its own scope
    /// names (`note_shadowing`), as the source needs no other renaming: binders of two methods,
    /// of sibling blocks, or one shadowing a binder it does not use keep their names, which the
    /// emitter numbers as the whole program's.
    fn unshadowed(&mut self, cv: &mut Conv, addr: Addr, name: Name) -> Name {
        if let Some(&entered) = cv.locals.get(&addr) {
            return entered;
        }
        let product = self.converts_product(cv);
        let taken = if product { cv.renamed.contains_key(&addr) } else { cv.locals.iter().any(|(&a, &n)| n == name && a != addr) };
        let renamed = if name == names::WILDCARD || !taken { name } else { self.interner.intern(&format!("{}$b{}", self.name_ref(name), addr)) };
        if product {
            cv.scope.push((renamed, addr));
            if renamed != name {
                cv.reader().binder_sources.insert(renamed, name);
            }
        }
        renamed
    }

    /// For a product's body, the binders of `binders` (by address and pickled name) that shadow
    /// a binder in scope which `within` (their scope's trees) or `types` names: those are renamed
    /// apart (`unshadowed`).
    fn note_shadowing(&mut self, cv: &mut Conv, binders: &[(Addr, NameRef)], within: &[&Term], types: &[&TType]) {
        if cv.scope.is_empty() || !self.converts_product(cv) {
            return;
        }
        for &(addr, n) in binders {
            let name = self.conv_name(cv, n);
            let Some(&(_, outer)) = cv.scope.iter().rev().find(|&&(m, a)| m == name && a != addr) else { continue };
            // A local class's parameter is named in its parents' calls as `C.this.x`.
            let this_ref = cv.parent_params.as_ref().filter(|(_, ps)| ps.contains(&name)).map(|&(at, _)| at);
            let text = self.name_str(name).to_string();
            let by_this = |t: &Term| this_ref.map_or(false, |at| term_names_this_member(&cv.tasty, t, at, &text));
            if types.iter().any(|t| type_mentions(t, outer)) || within.iter().any(|t| term_mentions(t, outer) || by_this(t)) {
                cv.renamed.insert(addr, ());
            }
        }
    }

    /// A local method, or a member of a local class, as a definition whose parameters and type
    /// parameters are spelled out; the body refers to them by name.
    fn conv_def(&mut self, cv: &mut Conv, sig: &DefSig, rhs: Option<&Term>, member: bool) -> DefId {
        cv.at(sig.addr, |cv| self.conv_def_now(cv, sig, rhs, member))
    }

    fn conv_def_now(&mut self, cv: &mut Conv, sig: &DefSig, rhs: Option<&Term>, member: bool) -> DefId {
        let name = self.conv_name(cv, sig.name);
        cv.locals.insert(sig.addr, name);
        if !cv.product {
            return self.conv_def_scoped(cv, sig, rhs, member, name);
        }
        {
            cv.scope.push((name, sig.addr));
            let params: Vec<(Addr, NameRef)> = sig.clauses.iter().flat_map(|c| match c {
                Clause::Terms(ps) => ps.iter().map(|p| (p.addr, p.name)).collect(),
                Clause::Types(_) => Vec::new(),
            }).collect();
            let types: Vec<&TType> = sig.clauses.iter().flat_map(|c| match c {
                Clause::Terms(ps) => ps.iter().map(|p| &p.ty).collect(),
                Clause::Types(ps) => ps.iter().map(|p| &p.info).collect::<Vec<_>>(),
            }).chain(std::iter::once(&sig.ret)).collect();
            self.note_shadowing(cv, &params, rhs.as_slice(), &types);
        }
        let mark = cv.scope.len();
        let d = self.conv_def_scoped(cv, sig, rhs, member, name);
        cv.scope.truncate(mark);
        d
    }

    fn conv_def_scoped(&mut self, cv: &mut Conv, sig: &DefSig, rhs: Option<&Term>, member: bool, name: Name) -> DefId {
        let mut tparams = Vec::new();
        let mut clauses = Vec::new();
        for clause in &sig.clauses {
            match clause {
                Clause::Types(ps) => {
                    for p in ps {
                        let n = self.conv_name(cv, p.name);
                        cv.local_types.insert(p.addr, n);
                        let arity = super::types::hk_arity(&p.info);
                        let (lower, upper) = match &p.info {
                            TType::Bounds(lo, hi) if arity == 0 => (Some(self.conv_type(cv, lo)), Some(self.conv_type(cv, hi))),
                            _ => (None, None),
                        };
                        tparams.push(TypeParam { name: n, span: cv.span, variance: 0, arity, hk_variances: Vec::new(), upper, lower, context_bounds: Vec::new(), evidence_names: Vec::new(), annots: Vec::new() });
                    }
                }
                Clause::Terms(ps) => {
                    let mut params = Vec::new();
                    let is_using = ps.iter().any(|p| p.flags.has(tags::GIVEN) || p.flags.has(tags::IMPLICIT));
                    for p in ps {
                        let n = self.conv_name(cv, p.name);
                        let n = self.unshadowed(cv, p.addr, n);
                        cv.locals.insert(p.addr, n);
                        cv.local_val_types.insert(p.addr, p.ty.clone());
                        if is_using {
                            cv.local_givens.insert(p.addr, ());
                        }
                        let ty = self.conv_param_type(cv, &p.ty);
                        let pm = if p.flags.has(tags::INLINE) { mods::INLINE } else { 0 };
                        params.push(Param { name: n, span: cv.span, ty, default: None, mods: pm, annots: ListRef::EMPTY });
                    }
                    let is_implicit = ps.iter().any(|p| p.flags.has(tags::IMPLICIT) && !p.flags.has(tags::GIVEN));
                    clauses.push(ParamClause { params, is_using, is_implicit });
                }
            }
        }
        let ret = Some(self.conv_type(cv, &sig.ret));
        let body = rhs.map(|r| self.conv_expr(cv, r));
        let mut m = 0;
        if sig.mods.flags.has(tags::INLINE) {
            m |= mods::INLINE;
        }
        if sig.mods.flags.has(tags::TRANSPARENT) {
            m |= mods::TRANSPARENT;
        }
        if member {
            m |= flag_mods(sig.mods.flags) & (mods::OVERRIDE | mods::FINAL | mods::IMPLICIT | mods::GIVEN | qualified_access(&sig.mods));
        }
        // An extension's type parameters and receiver's clause come first, as its pickle has
        // them.
        let is_extension = sig.mods.flags.has(tags::EXTENSION);
        let ext_tparams = if is_extension { sig.clauses.iter().take_while(|c| matches!(c, Clause::Types(_))).map(|c| match c { Clause::Types(ps) => ps.len(), _ => 0 }).sum() } else { 0 };
        let ext_clauses = if is_extension && !clauses.is_empty() { 1 } else { 0 };
        let fun = FunDef { tparams, clauses, ret, body, ext_tparams: ext_tparams as u8, ext_clauses, is_extension, ext_group: 0 };
        let d = cv.ast.add_def(Def { name, span: cv.span, mods: m, annots: Vec::new(), kind: DefKind::Fun(Box::new(fun)) });
        cv.def_at.push((d, sig.addr));
        d
    }

    fn conv_cases(&mut self, cv: &mut Conv, cases: &[Case]) -> ListRef {
        let mut out = Vec::with_capacity(cases.len());
        for c in cases {
            let mark = cv.scope.len();
            if self.converts_product(cv) && !cv.scope.is_empty() {
                let mut binds: Vec<(Addr, NameRef)> = Vec::new();
                let mut walk = crate::tasty::terms::Walk {
                    f: &mut |t: &Term, _: &crate::tasty::terms::Scope| {
                        if let TermKind::Bind { addr, name, .. } = &t.kind {
                            binds.push((*addr, *name));
                        }
                    },
                };
                walk.term(&c.pat, crate::tasty::terms::Scope::default());
                let within: Vec<&Term> = c.guard.iter().chain(std::iter::once(&c.body)).collect();
                self.note_shadowing(cv, &binds, &within, &[]);
            }
            let outer_named = cv.named_pattern_vars.clone();
            let outer_own = std::mem::take(&mut cv.case_pattern_vars);
            // Every type variable the pattern binds, named (`Box[t]`) or anonymous (scalac's `_$N`
            // for `Box[_]`), is one of the case's.
            if !cv.in_inline_match {
                let mut binds = Vec::new();
                pattern_type_binds(&cv.tasty, &c.pat, &mut binds);
                for addr in binds {
                    cv.named_pattern_vars.insert(addr, ());
                    cv.case_pattern_vars.push(addr);
                    if super::declared::dump_path().is_some() {
                        self.dump_binder(cv, addr);
                    }
                }
            }
            let pat = self.conv_pat(cv, &c.pat);
            let guard = c.guard.as_ref().map(|g| self.conv_expr(cv, g));
            let body = self.conv_expr(cv, &c.body);
            cv.named_pattern_vars = outer_named;
            cv.case_pattern_vars = outer_own;
            out.push(CaseClause { pat, guard, body });
            cv.scope.truncate(mark);
        }
        push_list(&mut cv.ast.cases, &out)
    }

    /// `summonFrom { cases }` is pickled as a match without a selector; the expander takes the
    /// call with its cases.
    fn conv_summon_from(&mut self, cv: &mut Conv, cases: &[Case]) -> ExprId {
        let l = self.conv_cases(cv, cases);
        let scrutinee = cv.expr(Expr::Ident(names::CASE_PARAM));
        let body = cv.expr(Expr::Match(scrutinee, l));
        let param = LambdaParam { name: names::CASE_PARAM, span: cv.span, ty: None, implicit: false, contextual: false };
        let params = push_list(&mut cv.ast.lambda_params, &[param]);
        let lambda = cv.expr(Expr::Lambda(params, body));
        let mut path = cv.expr(Expr::Ident(names::ROOT));
        for seg in ["scala", "compiletime", "summonFrom"] {
            let n = self.interner.intern(seg);
            path = cv.expr(Expr::Select(path, n));
        }
        let args = push_list(&mut cv.ast.expr_lists, &[lambda]);
        cv.expr(Expr::Apply(path, args))
    }

    /// `case '[List[t]]` as a TASTy 28.3 file pickles it: an `unapply` of
    /// `QuoteMatching.TypeMatch` whose pattern type is `Type[List[t]]`, the type variables its
    /// `BIND`s; later pickles write a `QUOTEPATTERN`. The pattern is the type, with the
    /// variables as the type variables of a source quote pattern.
    fn conv_type_match_pat(&mut self, cv: &mut Conv, fun: &Term, implicits: &[Term], pats: &[Term], ty: &TType) -> Option<PatId> {
        let head = match &fun.kind {
            TermKind::TypeApply(f, ..) => &**f,
            _ => fun,
        };
        let (qual, n) = match &head.kind {
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) => (&**q, *n),
            _ => return None,
        };
        if cv.tasty.simple(cv.tasty.source_name(n)) != Some("unapply") {
            return None;
        }
        let on_type_match = match &qual.kind {
            TermKind::Select(_, m) | TermKind::SelectIn(_, m, ..) => cv.tasty.simple(cv.tasty.source_name(*m)) == Some("TypeMatch"),
            _ => false,
        };
        if !on_type_match {
            return None;
        }
        let TType::Applied(_, args) = ty else { return None };
        let [inner] = args.as_slice() else { return None };
        let tasty = cv.tasty.clone();
        let mut binds: Vec<(Addr, String)> = Vec::new();
        collect_type_binds(&tasty, inner, &mut binds);
        let vars: Vec<(Addr, Name)> = binds.iter().map(|(a, text)| (*a, self.interner.intern(text))).collect();
        for &(addr, name) in &vars {
            cv.local_types.insert(addr, name);
            cv.quote_type_vars.insert(addr, ());
        }
        // The `Type[t]` binders of the variables (`Tuple1[Type[f]](f$given1)`).
        let mut givens: Vec<(Addr, Addr)> = Vec::new();
        collect_type_given_binds(&tasty, pats, &mut givens);
        for (bind, var) in givens {
            if let Some(&(_, name)) = vars.iter().find(|&&(a, _)| a == var) {
                cv.type_givens.insert(bind, name);
            }
        }
        // The shape is the argument of the `Type.of[shape]` the unapply takes, as scalac decodes
        // it; the result type is the shape intersected with the scrutinee's type. The shape
        // names its variables by fresh `@patternType` symbols of the same names.
        let (shape, vars) = match implicits.iter().map(|t| &t.kind).collect::<Vec<_>>().as_slice() {
            [TermKind::Apply(f, _)] => match &f.kind {
                TermKind::TypeApply(_, targs, ..) if targs.len() == 1 => {
                    let mut locals = Vec::new();
                    collect_local_types(&targs[0], &mut locals);
                    let mut all = vars.clone();
                    for addr in locals {
                        let name = Decoder::new(&tasty).name_at(addr).map(|n| self.interner.intern(&tasty.name(tasty.source_name(n))));
                        if let Some(&(_, v)) = vars.iter().find(|&&(_, v)| Some(v) == name) {
                            all.push((addr, v));
                        }
                    }
                    (&targs[0], all)
                }
                _ => (inner, vars),
            },
            _ => (inner, vars),
        };
        let t = self.conv_quote_pat_type(cv, shape, &vars);
        Some(cv.pat(Pat::QuoteType(t)))
    }

    /// `case '{ $x: F[t] }` as a TASTy 28.3 file pickles it: an `unapply` of
    /// `QuoteMatching.ExprMatch` whose using argument is the pattern quoted, `quote[T]({
    /// @patternType type t; patternHole[F[t]] })`, with a tuple of the `Type[t]` binders and
    /// then the holes' binders; later pickles write a `QUOTEPATTERN`. The pattern is the source
    /// form, `'{ ($x: F[t]) }`, its type variables named as the pattern types are.
    fn conv_expr_match_pat(&mut self, cv: &mut Conv, fun: &Term, implicits: &[Term], pats: &[Term]) -> Option<PatId> {
        let head = match &fun.kind {
            TermKind::TypeApply(f, ..) => &**f,
            _ => fun,
        };
        let (qual, n) = match &head.kind {
            TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) => (&**q, *n),
            _ => return None,
        };
        if cv.tasty.simple(cv.tasty.source_name(n)) != Some("unapply") {
            return None;
        }
        let on_expr_match = match &qual.kind {
            TermKind::Select(_, m) | TermKind::SelectIn(_, m, ..) => cv.tasty.simple(cv.tasty.source_name(*m)) == Some("ExprMatch"),
            _ => false,
        };
        if !on_expr_match {
            return None;
        }
        let [quoted] = implicits else { return None };
        let body = self.quoted_pattern_body(cv, quoted)?;
        let binders: Vec<Term> = match pats.iter().map(|t| &t.kind).collect::<Vec<_>>().as_slice() {
            [TermKind::Unapply { pats: inner, .. }] => inner.clone(),
            [TermKind::Typed(inner, ..)] if matches!(&inner.kind, TermKind::Unapply { .. }) => match &inner.kind {
                TermKind::Unapply { pats: inner, .. } => inner.clone(),
                _ => unreachable!(),
            },
            _ => Vec::new(),
        };
        let (stats, expr): (&[Stat], &Term) = match &body.kind {
            TermKind::Block(stats, e) => (stats, e),
            _ => (&[], body),
        };
        let tasty = cv.tasty.clone();
        let mut vars: Vec<(Addr, Name)> = Vec::new();
        for st in stats {
            match st {
                Stat::Type(sig) => {
                    let name = self.lname(&tasty, tasty.source_name(sig.name));
                    vars.push((sig.addr, name));
                }
                _ => return None,
            }
        }
        for &(addr, name) in &vars {
            cv.local_types.insert(addr, name);
            cv.quote_type_vars.insert(addr, ());
        }
        // The binders' `Type[t]` name the variables by the `BIND`s of the unapply's type
        // arguments, which are the pattern types by name.
        let mut givens: Vec<(Addr, Addr)> = Vec::new();
        collect_type_given_binds(&tasty, &binders, &mut givens);
        let mut holes = std::collections::VecDeque::new();
        for b in &binders {
            // A hole's binder ascribed its type, `(m: Expr[T])`.
            let b = match &b.kind {
                TermKind::Typed(inner, ..) if matches!((**inner).kind, TermKind::Bind { .. }) => &**inner,
                _ => b,
            };
            match &b.kind {
                TermKind::Bind { addr, name, .. } => {
                    if let Some(&(_, var)) = givens.iter().find(|&&(bind, _)| bind == *addr) {
                        let var_name = Decoder::new(&tasty).name_at(var).map(|n| self.lname(&tasty, tasty.source_name(n)));
                        if let Some(&(_, vname)) = vars.iter().find(|&&(a, vn)| a == var || Some(vn) == var_name) {
                            cv.local_types.insert(var, vname);
                            cv.quote_type_vars.insert(var, ());
                            cv.type_givens.insert(*addr, vname);
                            continue;
                        }
                    }
                    let n = self.conv_name(cv, *name);
                    cv.locals.insert(*addr, n);
                    holes.push_back(cv.pat(Pat::Bind(n, None)));
                }
                TermKind::Ident(w, _) if cv.tasty.simple(*w) == Some("_") => holes.push_back(cv.pat(Pat::Wildcard)),
                // An extractor in a hole, `${Varargs(xs)}`.
                TermKind::Unapply { .. } => {
                    let p = self.conv_pat(cv, b);
                    holes.push_back(p);
                }
                _ => return None,
            }
        }
        let outer = std::mem::replace(&mut cv.pattern_holes, holes);
        let e = self.conv_expr(cv, expr);
        cv.pattern_holes = outer;
        // A variable declared with a bound (`@patternType type t <: Product`) keeps it, as the
        // source pattern `'{ type t <: Product; ... }` declares it.
        let mut decls = Vec::new();
        for st in stats {
            if let Stat::Type(sig) = st {
                if let TType::Bounds(lo, hi) = &sig.rhs {
                    if !self.names_nothing(cv, lo) || !self.names_any(cv, hi) {
                        if let Some(d) = self.conv_type_alias(cv, sig) {
                            decls.push(Stmt::Def(d));
                        }
                    }
                }
            }
        }
        if decls.is_empty() {
            return Some(cv.pat(Pat::Quote(e)));
        }
        decls.push(Stmt::Expr(e));
        let l = push_list(&mut cv.ast.stmts, &decls);
        let block = cv.expr(Expr::Block(l));
        Some(cv.pat(Pat::Quote(block)))
    }

    /// The body of `quote[T](body).apply(q)`, the pattern an `ExprMatch` takes.
    fn quoted_pattern_body<'t>(&mut self, cv: &Conv, t: &'t Term) -> Option<&'t Term> {
        let TermKind::Apply(f, _) = &t.kind else { return None };
        let (TermKind::Select(inner, n) | TermKind::SelectIn(inner, n, ..)) = &f.kind else { return None };
        if cv.tasty.simple(cv.tasty.source_name(*n)) != Some("apply") {
            return None;
        }
        let TermKind::Apply(g, args) = &inner.kind else { return None };
        match (self.quoted_runtime_op(cv, g), args.as_slice()) {
            (Some("quote"), [body]) => Some(body),
            _ => None,
        }
    }

    /// `patternHole[T]` inside a quote pattern: the next hole, `($x: T)`.
    fn conv_pattern_hole(&mut self, cv: &mut Conv, f: &Term, targs: &[TType]) -> Option<ExprId> {
        if cv.pattern_holes.is_empty() {
            return None;
        }
        let n = match &f.kind {
            TermKind::Select(_, n) | TermKind::SelectIn(_, n, ..) | TermKind::Ident(n, _) | TermKind::Path(TType::TermRef(_, n)) => *n,
            _ => return None,
        };
        if cv.tasty.simple(cv.tasty.source_name(n)) != Some("patternHole") {
            return None;
        }
        let [t] = targs else { return None };
        let p = cv.pattern_holes.pop_front()?;
        let hole = cv.expr(Expr::SplicePat(p));
        let ty = self.conv_type(cv, t);
        Some(cv.expr(Expr::Typed(hole, ty)))
    }

    /// The type of a quote type pattern, its bound variables as `TypeVar`s.
    fn conv_quote_pat_type(&mut self, cv: &mut Conv, t: &TType, vars: &[(Addr, Name)]) -> TyExprId {
        match t {
            TType::LocalType(addr, _) => match vars.iter().find(|&&(a, _)| a == *addr) {
                Some(&(_, name)) => cv.ty(TyExpr::TypeVar(name)),
                None => self.conv_type(cv, t),
            },
            TType::Applied(f, args) => {
                // The constructor as it stands, not a class applied to nothing
                // (`TransformerOverrides.Computed[toPath, cfg]`).
                let fe = if self.mentions_local_type(cv, f) {
                    self.conv_quote_pat_type(cv, f, vars)
                } else {
                    let mut cx = super::MapCx::new(cv.file);
                    let ctor = self.map_type_ctor(&mut cx, f);
                    cv.ty(TyExpr::Resolved(ctor))
                };
                let items: Vec<TyExprId> = args.iter().map(|a| self.conv_quote_pat_type(cv, a, vars)).collect();
                let l = push_list(&mut cv.ast.ty_lists, &items);
                cv.ty(TyExpr::Apply(fe, l))
            }
            other => self.conv_type(cv, other),
        }
    }

    fn conv_pat(&mut self, cv: &mut Conv, t: &Term) -> PatId {
        let p = cv.at(t.at, |cv| self.conv_pat_now(cv, t));
        if cv.product && !cv.replay_tests.is_empty() && cv.replay_tests.contains_key(&t.at) {
            cv.reader().replay.test_pats.insert(p, ());
        }
        p
    }

    fn conv_pat_now(&mut self, cv: &mut Conv, t: &Term) -> PatId {
        match &t.kind {
            // A bare identifier in a pattern is the wildcard or a stable identifier (`case
            // Identity =>` of a case object), never a binder, which is a `Bind`.
            TermKind::Ident(n, ty) => {
                let name = self.conv_name(cv, *n);
                if name == names::WILDCARD {
                    cv.pat(Pat::Wildcard)
                } else if cv.tasty.simple(*n) == Some("_*") {
                    let w = cv.pat(Pat::Wildcard);
                    cv.pat(Pat::Rest(w))
                } else {
                    let e = match ty {
                        TType::TermRef(..) | TType::LocalTerm(..) => self.conv_path(cv, ty),
                        _ => cv.expr(Expr::Ident(name)),
                    };
                    cv.pat(Pat::StableId(e))
                }
            }
            TermKind::Bind { addr, name, ty, body, .. } => {
                let n = self.conv_name(cv, *name);
                let n = self.unshadowed(cv, *addr, n);
                cv.locals.insert(*addr, n);
                // The binder's declared type, for `m.O` over a wildcard-typed `m`.
                cv.local_val_types.insert(*addr, (**ty).clone());
                let inner = match &body.kind {
                    TermKind::Ident(w, _) if cv.tasty.simple(*w) == Some("_") => None,
                    _ => Some(self.conv_pat(cv, body)),
                };
                // `xs*`, spelt `xs @ (_*: Seq[A])` in a TASTy pattern.
                if let Some(i) = inner.filter(|&i| matches!(cv.ast.pat(i), Pat::Rest(w) if matches!(cv.ast.pat(w), Pat::Wildcard))) {
                    let _ = i;
                    let bind = cv.pat(Pat::Bind(n, None));
                    return cv.pat(Pat::Rest(bind));
                }
                cv.pat(Pat::Bind(n, inner))
            }
            // scalac ascribes an extractor pattern with the class at the type variables it
            // bound (`Done[A$9](a)`); the constructor pattern instantiates the class from the
            // scrutinee itself, which is where those variables come from, unless the body names
            // a variable, which the ascription then binds.
            TermKind::Typed(inner, ty, ..) if matches!((**inner).kind, TermKind::Unapply { .. }) && self.mentions_local_type(cv, ty) && !self.mentions_named_var(cv, ty) => {
                self.conv_pat(cv, inner)
            }
            TermKind::Typed(inner, ty, ..) => {
                // A binder ascribed a type has it as a val does: `it.A` for `it: Iterable[_]`
                // reads as the wildcard's bound.
                if let TermKind::Bind { addr, .. } = &inner.kind {
                    cv.local_val_types.insert(*addr, ty.clone());
                }
                let p = self.conv_pat(cv, inner);
                // `xs*` ascribed its sequence type stays the rest pattern it is.
                if matches!(cv.ast.pat(p), Pat::Rest(_)) {
                    return p;
                }
                // A tuple pattern past 22 elements, which scalac ascribes its extractor's
                // `TupleXXL`: the tuple pattern tests that itself, on the scrutinee's elements.
                if matches!(cv.ast.pat(p), Pat::Tuple(_)) && matches!(&inner.kind, TermKind::Unapply { .. }) && matches!(ty, TType::TypeRef(prefix, n) if self.is_tuple_xxl_path(cv, prefix, *n)) {
                    return p;
                }
                // A pattern ascribed the repeated type is a rest, whatever its identifier: scalac
                // spells an anonymous `_*` as `_: <repeated>[T]`.
                if repeated_element(&cv.tasty, ty).is_some() {
                    return cv.pat(Pat::Rest(p));
                }
                let outer = std::mem::replace(&mut cv.in_inline_pattern, cv.in_inline_match);
                let t = self.conv_type(cv, ty);
                let bounds = self.binder_bounds(cv, ty);
                cv.in_inline_pattern = outer;
                let typed = cv.pat(Pat::Typed(p, t));
                if !bounds.is_empty() {
                    cv.reader().binder_bounds.insert(typed, bounds);
                }
                typed
            }
            TermKind::Const(c) => {
                let e = self.conv_const(cv, c);
                cv.pat(Pat::Lit(e))
            }
            TermKind::Path(p) => {
                let e = self.conv_path(cv, p);
                cv.pat(Pat::StableId(e))
            }
            // `case JsonCursor.Identity =>`: a stable identifier selected from a path; `case
            // this =>` (`Zero.this` of scala-library's `TreeSeqMap.Zero.equals`).
            TermKind::Select(..) | TermKind::SelectIn(..) | TermKind::QualThis(_) => {
                let e = self.conv_expr(cv, t);
                cv.pat(Pat::StableId(e))
            }
            TermKind::Alternative(alts) => {
                let items: Vec<PatId> = alts.iter().map(|a| self.conv_pat(cv, a)).collect();
                let l = push_list(&mut cv.ast.pat_lists, &items);
                cv.pat(Pat::Alt(l))
            }
            TermKind::Unapply { fun, pats, ty, implicits } => {
                if let Some(p) = self.conv_type_match_pat(cv, fun, implicits, pats, ty) {
                    return p;
                }
                if let Some(p) = self.conv_expr_match_pat(cv, fun, implicits, pats) {
                    return p;
                }
                let mut head: &Term = fun;
                loop {
                    match &head.kind {
                        TermKind::TypeApply(f, ..) | TermKind::Apply(f, _) => head = f,
                        _ => break,
                    }
                }
                let subs: Vec<PatId> = pats.iter().map(|p| self.conv_pat(cv, p)).collect();
                let l = push_list(&mut cv.ast.pat_lists, &subs);
                let extractor = match &head.kind {
                    TermKind::Select(q, n) | TermKind::SelectIn(q, n, ..) => {
                        if let TermKind::Path(TType::TermRef(prefix, tn)) = &q.kind {
                            let name = cv.tasty.simple(cv.tasty.source_name(*n));
                            if self.is_scala_tuple_path(cv, prefix, *tn) && name == Some("unapply") {
                                return cv.pat(Pat::Tuple(l));
                            }
                            // A tuple pattern past 22 elements, which scalac and teq's writer spell
                            // `TupleXXL(p1, ..., pn)`: its bare binders (and scalac's wildcards) are
                            // declared with the elements' types, where those of a `TupleXXL(..)` the
                            // source wrote are `Any`s. A binder of a pattern (`a @ (_: Int)`) is
                            // declared with what the pattern gives in either, which tells neither.
                            if name == Some("unapplySeq") && self.is_tuple_xxl_path(cv, prefix, *tn) && pats.iter().any(|p| self.declares_element(cv, p)) {
                                return cv.pat(Pat::Tuple(l));
                            }
                        }
                        self.conv_expr(cv, q)
                    }
                    _ => cv.refuse("an extractor of that shape"),
                };
                cv.pat(Pat::Ctor(extractor, l))
            }
            TermKind::QuotePattern { body, bindings, .. } => self.conv_quote_pattern(cv, body, bindings),
            // A literal pattern of the refusal, so that typing the pattern reports it: a
            // pattern read as a wildcard would match what the source's does not.
            _ => {
                let e = cv.refuse(format!("{} in a pattern", tag_name(t)));
                cv.pat(Pat::Lit(e))
            }
        }
    }

    /// `QUOTEPATTERN body quotes patType bindings*` (TASTy 28.4 and later): the source's quote
    /// pattern, `'{ .. }` with its `SPLICEPATTERN`s as `$x` holes, or `'[T]` for a type body. The
    /// bindings are the pattern's type variables, `BIND t TYPEBOUNDS IDENT _`, which the body
    /// names by address; a variable the typer would not take for one by its name (an upper-case
    /// one) or that has bounds is declared in front of the body, as the source declares it.
    fn conv_quote_pattern(&mut self, cv: &mut Conv, body: &Term, bindings: &[Stat]) -> PatId {
        let tasty = cv.tasty.clone();
        let mut vars: Vec<(Addr, Name)> = Vec::new();
        let mut bounded: Vec<(Name, TType)> = Vec::new();
        for b in bindings {
            let Stat::Expr(Term { kind: TermKind::Bind { addr, name, ty, .. }, .. }) = b else {
                let e = cv.refuse("a quote pattern's binding of that shape");
                return cv.pat(Pat::Lit(e));
            };
            let n = self.lname(&tasty, tasty.source_name(*name));
            vars.push((*addr, n));
            cv.local_types.insert(*addr, n);
            cv.quote_type_vars.insert(*addr, ());
            let trivial = match &**ty {
                TType::Bounds(lo, hi) => self.names_nothing(cv, lo) && self.names_any(cv, hi),
                _ => true,
            };
            let lower_case = self.name_str(n).starts_with(|c: char| c.is_lowercase());
            if !trivial || !lower_case {
                bounded.push((n, (**ty).clone()));
            }
        }
        if let TermKind::Path(t) = &body.kind {
            let t = self.conv_quote_pat_type(cv, t, &vars);
            return cv.pat(Pat::QuoteType(t));
        }
        let e = self.conv_expr(cv, body);
        if bounded.is_empty() {
            return cv.pat(Pat::Quote(e));
        }
        let mut stmts = Vec::new();
        for (name, ty) in bounded {
            let (lower, upper) = match &ty {
                TType::Bounds(lo, hi) => (
                    (!self.names_nothing(cv, lo)).then(|| self.conv_type(cv, lo)),
                    (!self.names_any(cv, hi)).then(|| self.conv_type(cv, hi)),
                ),
                _ => (None, None),
            };
            let d = cv.ast.add_def(Def { name, span: cv.span, mods: 0, annots: Vec::new(), kind: DefKind::TypeAlias { tparams: Vec::new(), rhs: None, lower, upper } });
            stmts.push(Stmt::Def(d));
        }
        stmts.push(Stmt::Expr(e));
        let l = push_list(&mut cv.ast.stmts, &stmts);
        let block = cv.expr(Expr::Block(l));
        cv.pat(Pat::Quote(block))
    }
}

/// The access modifier of a member that a body keeps: a plain `private`, since `private[p]`
/// is reachable from all of `p`, which the library was checked for.
fn qualified_access(m: &crate::tasty::tree::Mods) -> Mods {
    if m.within.is_some() { 0 } else { mods::PRIVATE }
}

fn kind_word(kind: ClassKind) -> &'static str {
    match kind {
        ClassKind::Object => "object",
        ClassKind::Trait => "trait",
        ClassKind::Enum => "enum",
        _ => "class",
    }
}

/// The type variables a quote type pattern binds: the `BIND`s the pattern type names.
/// The binders of a quote type pattern's `Type[t]` givens, each with the address of its
/// variable `t`: `Bind(f$given1: Type[f])` inside the `Tuple1(..)` the match tests.
/// A hole's binder `a: Expr[t]` has the same shape and is none of them.
fn collect_type_given_binds(tasty: &TastyFile, pats: &[Term], out: &mut Vec<(Addr, Addr)>) {
    for p in pats {
        match &p.kind {
            TermKind::Bind { addr, ty, body, .. } => {
                if let TType::Applied(ctor, args) = &**ty {
                    if let ([TType::LocalType(var, _)], true) = (args.as_slice(), names_quoted_type(tasty, ctor)) {
                        out.push((*addr, *var));
                    }
                }
                collect_type_given_binds(tasty, std::slice::from_ref(body), out);
            }
            TermKind::Unapply { pats, .. } => collect_type_given_binds(tasty, pats, out),
            TermKind::Typed(inner, ..) => collect_type_given_binds(tasty, std::slice::from_ref(inner), out),
            _ => {}
        }
    }
}

/// Whether `t` is `scala.quoted.Type`.
fn names_quoted_type(tasty: &TastyFile, t: &TType) -> bool {
    let TType::TypeRef(prefix, n) = t else { return false };
    if tasty.simple(tasty.source_name(*n)) != Some("Type") {
        return false;
    }
    match &**prefix {
        TType::Package(p) => tasty.name(*p) == "scala.quoted",
        TType::TermRef(q, m) => tasty.simple(tasty.source_name(*m)) == Some("quoted") && matches!(&**q, TType::Package(p) if tasty.name(*p) == "scala"),
        _ => false,
    }
}

fn collect_local_types(t: &TType, out: &mut Vec<Addr>) {
    match t {
        TType::LocalType(addr, _) if !out.contains(addr) => out.push(*addr),
        TType::Applied(f, args) => {
            collect_local_types(f, out);
            for a in args {
                collect_local_types(a, out);
            }
        }
        TType::And(a, b) | TType::Or(a, b) => {
            collect_local_types(a, out);
            collect_local_types(b, out);
        }
        TType::Alias(a) | TType::BoundedAlias(_, a) | TType::Annotated(a, _) => collect_local_types(a, out),
        _ => {}
    }
}

fn collect_type_binds(tasty: &TastyFile, t: &TType, out: &mut Vec<(Addr, String)>) {
    let walk = |t: &TType, out: &mut Vec<(Addr, String)>| collect_type_binds(tasty, t, out);
    match t {
        TType::LocalType(addr, _) => {
            if Decoder::new(tasty).tag_at(*addr) == tags::BIND && !out.iter().any(|(a, _)| a == addr) {
                if let Some(n) = Decoder::new(tasty).name_at(*addr) {
                    out.push((*addr, tasty.name(tasty.source_name(n)).to_string()));
                }
            }
        }
        TType::Applied(f, args) => {
            walk(f, out);
            for a in args {
                walk(a, out);
            }
        }
        TType::And(a, b) | TType::Or(a, b) => {
            walk(a, out);
            walk(b, out);
        }
        TType::Alias(a) | TType::BoundedAlias(_, a) | TType::Annotated(a, _) => walk(a, out),
        _ => {}
    }
}

/// The `patternHole[T]`s of a quote pattern inside `t`.
fn count_pattern_holes(tasty: &TastyFile, t: &Term) -> usize {
    let mut n = 0;
    let mut walk = crate::tasty::terms::Walk {
        f: &mut |term: &Term, _scope: &crate::tasty::terms::Scope| {
            if let TermKind::TypeApply(g, ..) = &term.kind {
                let name: Option<NameRef> = match &g.kind {
                    TermKind::Select(_, m) | TermKind::SelectIn(_, m, ..) | TermKind::Ident(m, _) | TermKind::Path(TType::TermRef(_, m)) => Some(m.clone()),
                    _ => None,
                };
                if name.map_or(false, |m| tasty.simple(tasty.source_name(m)) == Some("patternHole")) {
                    n += 1;
                }
            }
        },
    };
    walk.term(t, crate::tasty::terms::Scope::default());
    n
}

/// The addresses of the local methods that the closures of `t` are over.
fn lambda_targets(t: &Term) -> Vec<Addr> {
    let mut out = Vec::new();
    let mut walk = crate::tasty::terms::Walk {
        f: &mut |term: &Term, _scope: &crate::tasty::terms::Scope| {
            if let TermKind::Lambda(meth, _) = &term.kind {
                if let TermKind::Path(TType::LocalTerm(addr, _)) = &meth.kind {
                    out.push(*addr);
                }
            }
        },
    };
    walk.term(t, crate::tasty::terms::Scope::default());
    out
}

/// Stands on `Conv::this_chain` for a local class, which has no id while it is converted.
const LOCAL_CLASS: ClassId = ClassId(u32::MAX);

enum PathScope {
    Pkg(PkgId),
    Module(ClassId),
}

/// `teq tasty --inline-bodies`: every inline method of the jars, converted and expanded with
/// its parameters bound to placeholders, and what stopped the ones that did not expand.
pub fn inline_bodies_report(paths: &[String]) -> Result<String, String> {
    super::with_everything_loaded(paths, |typer| {
        let mut converted = 0;
        let mut expanded = 0;
        let mut refused: Vec<(String, String)> = Vec::new();
        let mut failed: Vec<(String, String)> = Vec::new();
        let syms: Vec<SymId> = (0..typer.syms.syms.len() as u32).map(SymId).collect();
        for s in syms {
            let info = typer.syms.sym(s);
            if info.mods & mods::INLINE == 0 || info.kind != SymKind::Def || !typer.loaded.as_ref().unwrap().syms.contains_key(&s) {
                continue;
            }
            let owner = info.owner;
            let name = typer.method_description(s);
            let sig = typer.sig_arc(s);
            let Owner::Class(c) = owner else { continue };
            if !typer.convert_library_class(c) || typer.syms.sym(s).def.is_none() {
                let why = typer.loaded.as_ref().unwrap().bodies.refused.iter().rev().map(|(_, why)| why.clone()).next().unwrap_or_default();
                refused.push((name, why));
                continue;
            }
            converted += 1;
            let subst: Subst = sig.tparams.iter().map(|&p| (p, typer.syms.tparam(p).upper)).collect();
            let args: Vec<TExprId> = sig.clauses.iter().flat_map(|c| c.params.iter()).map(|_| typer.prog.add(TExpr::Unit)).collect();
            let ret = typer.types.subst(sig.ret, &subst);
            let call = super::super::apply::MethodCall { recv: None, sym: s, owner_subst: Vec::new(), ext_recv: None, prefix: None };
            let mark = typer.diags.items.len();
            typer.expand_inline(&call, &sig, &subst, &args, ret, Span::default(), None);
            let errors: Vec<String> = typer.diags.items[mark..].iter().filter(|d| !d.is_warning).map(|d| d.msg.lines().next().unwrap_or("").to_string()).collect();
            typer.diags.items.truncate(mark);
            if errors.is_empty() {
                expanded += 1;
            } else {
                failed.push((name, errors[0].clone()));
            }
        }
        let mut out = format!(
            "{} inline methods: {} converted, {} expand with placeholder arguments, {} refused by the converter, {} fail to type\n",
            converted + refused.len(),
            converted,
            expanded,
            refused.len(),
            failed.len()
        );
        for (name, why) in &refused {
            out.push_str(&format!("  refused  {}: {}\n", name, why));
        }
        for (name, why) in &failed {
            out.push_str(&format!("  fails    {}: {}\n", name, why));
        }
        out
    })
}

/// The elements of a single vararg argument, `xs*` or none.
fn repeated_items(args: &[Term]) -> Option<&[Term]> {
    match args {
        [] => Some(&[]),
        [arg] => match &arg.kind {
            TermKind::Typed(inner, ..) => match &inner.kind {
                TermKind::Repeated(_, elems) => Some(elems),
                _ => None,
            },
            TermKind::Repeated(_, elems) => Some(elems),
            _ => None,
        },
        _ => None,
    }
}

/// The `BIND`s a pattern's type introduces or names, by address, in order.
fn type_binds(tasty: &crate::tasty::TastyFile, t: &TType, out: &mut Vec<Addr>) {
    match t {
        TType::LocalType(addr, _) => {
            if Decoder::new(tasty).tag_at(*addr) == tags::BIND && !out.contains(addr) {
                out.push(*addr);
            }
        }
        TType::Applied(f, args) => {
            type_binds(tasty, f, out);
            for a in args {
                type_binds(tasty, a, out);
            }
        }
        TType::And(a, b) | TType::Or(a, b) | TType::Bounds(a, b) => {
            type_binds(tasty, a, out);
            type_binds(tasty, b, out);
        }
        TType::ByName(inner) | TType::Annotated(inner, _) | TType::Flexible(inner) => type_binds(tasty, inner, out),
        _ => {}
    }
}

/// The addresses of the `BIND`s of the type variables a pattern's ascriptions bind.
fn pattern_type_binds(tasty: &crate::tasty::TastyFile, pat: &Term, out: &mut Vec<Addr>) {
    match &pat.kind {
        TermKind::Typed(inner, ty, ..) => {
            type_binds(tasty, ty, out);
            pattern_type_binds(tasty, inner, out);
        }
        TermKind::Bind { body, .. } => pattern_type_binds(tasty, body, out),
        TermKind::Unapply { pats, .. } => {
            for p in pats {
                pattern_type_binds(tasty, p, out);
            }
        }
        TermKind::Alternative(alts) => {
            for a in alts {
                pattern_type_binds(tasty, a, out);
            }
        }
        _ => {}
    }
}

/// The method an extension call selects through its owner, its type arguments written before
/// the receiver (`IArray.map[T](arr)`), and the argument clauses up to the receiver's, in order.
fn extension_clauses(t: &Term) -> Option<(&Term, &[TType], Vec<&[Term]>)> {
    let mut clauses: Vec<&[Term]> = Vec::new();
    let mut head = t;
    while let TermKind::Apply(g, args) = &head.kind {
        clauses.push(args);
        head = g;
    }
    clauses.reverse();
    let (g, ext_targs): (&Term, &[TType]) = match &head.kind {
        TermKind::TypeApply(inner, ta, ..) if matches!(&inner.kind, TermKind::Select(..) | TermKind::SelectIn(..) | TermKind::Path(_)) => (inner, ta),
        _ => (head, &[]),
    };
    (!clauses.is_empty() && matches!(&g.kind, TermKind::Select(..) | TermKind::SelectIn(..) | TermKind::Path(_))).then_some((g, ext_targs, clauses))
}

fn term_param_count(sig: &DefSig) -> usize {
    sig.clauses.iter().map(|c| if let Clause::Terms(ps) = c { ps.len() } else { 0 }).sum()
}

/// The argument lists of a call, the first first.
/// A constructor call's argument list without its trailing default getters of the constructor
/// (`$lessinit$greater$default$N`, `N` the parameter's place among the call's, `before` the
/// parameters of the lists before this one).
fn own_defaults_trimmed<'t>(cv: &Conv, list: &'t [Term], before: usize) -> &'t [Term] {
    let mut end = list.len();
    while end > 0 {
        let mut a = &list[end - 1];
        while let TermKind::Apply(g, _) | TermKind::TypeApply(g, ..) = &a.kind {
            a = g;
        }
        let own = match &a.kind {
            TermKind::Select(_, d) | TermKind::SelectIn(_, d, ..) => match cv.tasty.names.get(cv.tasty.source_name(*d) as usize) {
                Some(TName::DefaultGetter(u, index)) => *index as usize + 1 == before + end && cv.tasty.simple(*u) == Some("<init>"),
                _ => false,
            },
            _ => false,
        };
        if !own {
            break;
        }
        end -= 1;
    }
    &list[..end]
}

fn call_lists(call: &Term) -> Vec<&[Term]> {
    let mut lists = Vec::new();
    let mut head = call;
    loop {
        match &head.kind {
            TermKind::Apply(f, args) => {
                lists.push(args.as_slice());
                head = f;
            }
            TermKind::TypeApply(f, ..) => head = f,
            _ => break,
        }
    }
    lists.reverse();
    lists
}

/// An argument that is one of a parent call's hoisted temporaries, positional or named: its
/// name, if named, and the temporary's index.
fn hoisted_arg(a: &Term, hoisted: &[(Addr, &Term)]) -> Option<(Option<NameRef>, usize)> {
    let (name, v) = match &a.kind {
        TermKind::NamedArg(n, v) => (Some(*n), &**v),
        _ => (None, a),
    };
    let TermKind::Path(TType::LocalTerm(addr, _)) = &v.kind else { return None };
    hoisted.iter().position(|(h, _)| h == addr).map(|i| (name, i))
}

/// A list's arguments in the order the source wrote them: the positional ones as they stand,
/// then the named ones, those of temporaries in the temporaries' order and the others after.
fn source_order<'t>(list: &'t [Term], hoisted: &[(Addr, &Term)]) -> Vec<&'t Term> {
    let named = |a: &Term| matches!(a.kind, TermKind::NamedArg(..));
    let mut out: Vec<&Term> = list.iter().filter(|a| !named(a)).collect();
    let mut temps: Vec<(usize, &Term)> = list.iter().filter(|a| named(a)).filter_map(|a| hoisted_arg(a, hoisted).map(|(_, i)| (i, a))).collect();
    temps.sort_by_key(|&(i, _)| i);
    out.extend(temps.into_iter().map(|(_, a)| a));
    out.extend(list.iter().filter(|a| named(a) && hoisted_arg(a, hoisted).is_none()));
    out
}

/// Whether a parent call's hoisted temporaries read back as its arguments: each of them is one
/// argument, and in the source's order of the lists (`source_order`) their indices rise, so
/// that reading each from its temporary evaluates them as the block does.
fn hoisted_order(lists: &[&[Term]], hoisted: &[(Addr, &Term)]) -> Option<()> {
    let mut next = 0;
    for list in lists {
        for a in source_order(list, hoisted) {
            if let Some((_, i)) = hoisted_arg(a, hoisted) {
                if i != next {
                    return None;
                }
                next += 1;
            }
        }
    }
    (next == hoisted.len()).then_some(())
}

/// Whether a type names the local term at `addr`.
fn type_mentions(t: &TType, addr: Addr) -> bool {
    let m = |t: &TType| type_mentions(t, addr);
    match t {
        TType::LocalTerm(a, prefix) => *a == addr || prefix.as_deref().map_or(false, m),
        TType::LocalType(_, prefix) => prefix.as_deref().map_or(false, m),
        TType::TypeRef(p, _) | TType::TermRef(p, _) | TType::This(p) | TType::Alias(p) | TType::ByName(p) | TType::Flexible(p) | TType::Rec(_, p) => m(p),
        TType::Applied(f, args) => m(f) || args.iter().any(m),
        TType::Bounds(a, b) | TType::BoundedAlias(a, b) | TType::And(a, b) | TType::Or(a, b) | TType::Annotated(a, b) | TType::Super(a, b) | TType::MatchCase(a, b) => m(a) || m(b),
        TType::Refined(p, members) => m(p) || members.iter().any(|(_, t)| t.as_ref().map_or(false, m)),
        TType::Lambda { params, result, .. } => m(result) || params.iter().any(|p| m(&p.info)),
        TType::Match { scrutinee, bound, cases } => m(scrutinee) || bound.as_deref().map_or(false, m) || cases.iter().any(|c| m(&c.pattern) || m(&c.body)),
        _ => false,
    }
}

/// Whether a tree names the local term at `addr`, as a reference or in a type it carries.
fn term_mentions(t: &Term, addr: Addr) -> bool {
    let mut found = false;
    let mut walk = crate::tasty::terms::Walk {
        f: &mut |t: &Term, _: &crate::tasty::terms::Scope| {
            if found {
                return;
            }
            let m = |ty: &TType| type_mentions(ty, addr);
            let sig_mentions = |sig: &DefSig| m(&sig.ret) || sig.clauses.iter().any(|c| match c {
                Clause::Terms(ps) => ps.iter().any(|p| m(&p.ty)),
                Clause::Types(ps) => ps.iter().any(|p| m(&p.info)),
            });
            let stats_mention = |stats: &[Stat]| stats.iter().any(|s| matches!(s, Stat::Val(sig, _) | Stat::Def(sig, _) if sig_mentions(sig)));
            found = match &t.kind {
                TermKind::Path(ty) | TermKind::Ident(_, ty) | TermKind::New(ty) | TermKind::QualThis(ty) | TermKind::Typed(_, ty, _) | TermKind::SelectIn(_, _, ty, _) | TermKind::Elided(ty) => m(ty),
                TermKind::TypeApply(_, tys, _) => tys.iter().any(m),
                TermKind::Repeated(ty, _) => m(ty),
                TermKind::SelectOuter { ty, .. } => m(ty),
                TermKind::Bind { ty, .. } | TermKind::Unapply { ty, .. } => m(ty),
                TermKind::Block(stats, _) | TermKind::Inlined { bindings: stats, .. } => stats_mention(stats),
                _ => false,
            };
        },
    };
    walk.term(t, crate::tasty::terms::Scope::default());
    found
}

/// Whether a tree selects the member `name` of the local class at `class` through its `this`
/// (`C.this.x`).
fn term_names_this_member(tasty: &crate::tasty::TastyFile, t: &Term, class: Addr, name: &str) -> bool {
    let mut found = false;
    let mut walk = crate::tasty::terms::Walk {
        f: &mut |t: &Term, _: &crate::tasty::terms::Scope| {
            if let TermKind::Path(TType::TermRef(prefix, n)) = &t.kind {
                if matches!(&**prefix, TType::This(inner) if matches!(&**inner, TType::LocalType(a, _) if *a == class)) && tasty.simple(tasty.source_name(*n)) == Some(name) {
                    found = true;
                }
            }
        },
    };
    walk.term(t, crate::tasty::terms::Scope::default());
    found
}
