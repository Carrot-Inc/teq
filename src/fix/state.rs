//! What the rules of a configuration's `file:` sources keep between documents (docs/FIX.md, "Patches, suppressions
//! and rounds"): the guard by which a project's rules run on several rule workers. A rule that keeps nothing gives a
//! document's patch from the document and its configuration alone, so an instance a rule worker gives the one-worker
//! run's output; one that keeps something has to see every document on one instance, in the run's order, as
//! scalafix's own run gives it them (one configured instance for a run, the files one after another: scalafix-cli's
//! `MainOps.run`).
//!
//! The guard reads the typed rules' program a rule worker has, before any code of a rule runs. It names the first
//! definition it sees that keeps something, and what it cannot tell of a definition's type it takes to hold state;
//! it is no proof that a rule it passes keeps nothing (what a rule reaches through a library's own state or by
//! reflection is not in its sources). Something is kept, anywhere in the sources, by
//! - a `var` that is a member of a class, a trait or an object, or stands at the top level;
//! - a `val`, a `lazy val`, a parameterless given or a class parameter (which a method of the class may read) whose
//!   type is not known to hold nothing that changes (`known`), and a by-name class parameter whatever its type; a
//!   function is known to hold nothing only where the member is written as a function literal, whose body is one
//!   call's, and a `SymbolMatcher` only where it is written as a call of the API's own makers;
//! - a `var`, or a value of such a type, defined outside every method's body and every function literal (in a
//!   member's initialiser or among a template's statements), or in a method whose result can carry what read it out
//!   of the call: a rule, a function, a matcher, a value of a type the guard does not know. A local of a method whose
//!   result is plain data lives for one call and is no state;
//! - an object of the sources over a class of a library that is not one of the bases that hold nothing.
//!
//! And a rule's output is its patches alone where it runs on several workers: one that names what writes to the
//! process's streams or to the file system, what a worker's interpreter keeps a sequence of (a random generator's
//! next number, an identity hash), or what makes a collection that carries a function (a map's default, a sorted
//! collection's ordering), is heard and answered in the documents' order on one worker only (`NAMES`).

use crate::ast::{DefKind, Expr, ImportSel, Stmt, TyExpr, NO_END};
use crate::intern::FxMap;
use crate::source::{FileId, Span};
use crate::symbols::{ClassKind, Owner, SymKind};
use crate::typer::Worker;
use crate::types::{ClassId, SymId, TParamId, Type, TypeId};

/// The definition that keeps something: its file, where its name stands, its name.
pub(crate) struct Kept {
    pub file: FileId,
    pub at: u32,
    pub name: String,
}

/// The rules' sources as the guard reads them: their files; the classes of theirs that another class of theirs
/// extends, an enum's cases aside (a value of such a class may be one of the other, which may hold anything); and,
/// read once for the whole of `kept` so that a class is asked once whatever leads to it, each class's answer on what
/// it inherits (`bases`) and the data classes that are not known to hold nothing (`settle`), as a keeper's type and
/// as a method's result.
struct Sources<'f> {
    files: &'f [FileId],
    extended: Vec<ClassId>,
    bases: FxMap<ClassId, bool>,
    /// The same without the rule API's types and the functions among the bases: a class that is data and nothing else.
    apart: FxMap<ClassId, bool>,
    unknown: [FxMap<ClassId, ()>; 2],
    /// Whether the sources define something called `SymbolMatcher` or import one from elsewhere than the rule API.
    own_matcher: bool,
}

/// The first definition of `files` (the rules' program's `file:` sources) that keeps something between documents, in
/// the files' order and then the text's; none where the guard sees none.
pub(crate) fn kept(w: &mut Worker<'_>, files: &[FileId]) -> Option<Kept> {
    let classes: Vec<ClassId> = (0..w.syms.classes.len()).map(|i| ClassId(i as u32)).filter(|&c| files.contains(&w.syms.class(c).file)).collect();
    let mut src = Sources { files, extended: Vec::new(), bases: FxMap::default(), apart: FxMap::default(), unknown: [FxMap::default(), FxMap::default()], own_matcher: false };
    src.own_matcher = classes.iter().any(|&c| w.name_ref(w.syms.class(c).name) == "SymbolMatcher")
        || w.syms.syms.iter().any(|info| files.contains(&info.file) && w.name_ref(info.name) == "SymbolMatcher")
        || files.iter().any(|&f| {
            let ast = w.ast(f);
            ast.imports.iter().chain(ast.local_imports.iter()).any(|imp| match imp.sel {
                ImportSel::Name(n, renamed) => {
                    let api = imp.path.iter().map(|p| w.name_ref(*p)).collect::<Vec<_>>() == ["teq", "fix"];
                    renamed.is_some_and(|r| w.name_ref(r) == "SymbolMatcher") || (w.name_ref(n) == "SymbolMatcher" && (renamed.is_some() || !api))
                }
                _ => false,
            })
        });
    for &k in &classes {
        let (kind, parents) = (w.syms.class(k).kind, w.syms.class(k).parents.clone());
        for p in parents {
            if let Type::Class(c, _) = w.types.get(p) {
                if kind != ClassKind::EnumCase && files.contains(&w.syms.class(c).file) {
                    src.extended.push(c);
                }
            }
        }
    }
    for &k in &classes {
        bases_of(w, k, &mut src, false);
        bases_of(w, k, &mut src, true);
    }
    settle(w, &classes, &mut src);
    // What lives for one call: the body of a method whose result is plain data, and a function literal that stands
    // in no method whose result can carry what it made.
    let mut methods: Vec<(SymId, FileId, Span)> = Vec::new();
    for i in 0..w.syms.syms.len() {
        let s = SymId(i as u32);
        let info = w.syms.sym(s);
        if !files.contains(&info.file) || !matches!(info.kind, SymKind::Def | SymKind::Given) {
            continue;
        }
        let Some(def) = info.def else { continue };
        let ast = w.ast(info.file);
        if let DefKind::Fun(fun) = &ast.def(def).kind {
            if let Some(body) = fun.body {
                methods.push((s, info.file, ast.expr_span(body)));
            }
        }
    }
    let mut plain: Vec<(FileId, Span)> = Vec::new();
    let mut carrying: Vec<(FileId, Span)> = Vec::new();
    for &(s, file, span) in &methods {
        let ret = w.sig_arc(s).ret;
        if known(w, ret, &src, true) {
            plain.push((file, span));
        } else {
            carrying.push((file, span));
        }
    }
    let mut literals: Vec<(FileId, Span)> = Vec::new();
    for &f in files {
        let ast = w.ast(f);
        // A body no method's signature was read for (a constructor's) is one call's too.
        for d in &ast.defs {
            if let DefKind::Fun(fun) = &d.kind {
                if let Some(body) = fun.body {
                    let span = ast.expr_span(body);
                    if !methods.iter().any(|&(_, g, s)| g == f && s == span) {
                        plain.push((f, span));
                    }
                }
            }
        }
        for (i, e) in ast.exprs.iter().enumerate() {
            if matches!(e, Expr::Lambda(..) | Expr::PolyLambda(..)) {
                literals.push((f, ast.expr_spans[i]));
            }
        }
    }
    // Where the sources write a definition's name: a definition's own, a pattern's binder.
    let mut written: FxMap<(FileId, u32), ()> = FxMap::default();
    for &f in files {
        let ast = w.ast(f);
        written.extend(ast.defs.iter().map(|d| ((f, d.span.start), ())));
        written.extend(ast.pats.iter().enumerate().filter(|(_, p)| matches!(p, crate::ast::Pat::Bind(..))).map(|(i, _)| ((f, ast.pat_spans[i].start), ())));
    }
    let within = |spans: &[(FileId, Span)], f: FileId, at: u32| spans.iter().any(|&(g, s)| g == f && s.start <= at && at < s.end);
    let in_call = |f: FileId, at: u32| within(&plain, f, at) || (within(&literals, f, at) && !within(&carrying, f, at));
    let mut found: Vec<(usize, u32, String)> = Vec::new();
    for i in 0..w.syms.syms.len() {
        let s = SymId(i as u32);
        let (file, span, kind, owner) = {
            let info = w.syms.sym(s);
            (info.file, info.span, info.kind, info.owner)
        };
        let local = owner == Owner::Local;
        let Some(order) = files.iter().position(|&f| f == file) else { continue };
        if local && in_call(file, span.start) {
            continue;
        }
        // A value of a data class is a part of the class's values, read with the class wherever one is kept
        // (`settle`): by itself it keeps nothing.
        if let (SymKind::Val, Owner::Class(c)) = (kind, owner) {
            if data_apart(w, c, &src) {
                continue;
            }
        }
        // What the text does not define where it stands is no definition of the sources: a name the compiler made;
        // and a local value that stands elsewhere than at a definition's or a pattern's name, which is what an inline
        // call binds a parameter to (it carries the call's place, in the method's file) or a `for`'s binder (a
        // function's parameter, at the comprehension).
        let mut name = w.name_ref(w.syms.sym(s).name).to_string();
        let stands = written.contains_key(&(file, span.start)) && spelled(w, file, span, &name);
        if (name.contains('$') || (local && kind == SymKind::Val)) && !stands {
            continue;
        }
        // A given without a name is told as the word the text has.
        if kind == SymKind::Given && !stands {
            name = "given".to_string();
        }
        let keeps = match kind {
            SymKind::Var => true,
            SymKind::Val => !value_is_known(w, s, &src) && !(!local && written_as_function(w, s)) && !written_as_matcher(w, s, &src),
            // A given without parameters is a value; one with parameters is a method.
            SymKind::Given => w.sig_arc(s).clauses.is_empty() && !value_is_known(w, s, &src),
            _ => false,
        };
        if keeps {
            found.push((order, span.start, name));
        }
    }
    for &c in &classes {
        let (order, at, name) = {
            let info = w.syms.class(c);
            (files.iter().position(|&f| f == info.file).unwrap_or(0), info.span.start, info.name)
        };
        // A data class's parameters are its members (`data_apart`).
        if data_apart(w, c, &src) {
            continue;
        }
        // The parameters of the sources' other classes; a by-name one is a function of who made the instance.
        let params: Vec<(SymId, TypeId, bool)> = w.syms.class(c).ctor.iter().flat_map(|clause| clause.params.iter().map(|p| (p.sym, p.ty, p.by_name))).collect();
        for (sym, ty, by_name) in params {
            if by_name || !known(w, ty, &src, false) {
                let (pfile, pspan, pname) = {
                    let info = w.syms.sym(sym);
                    (info.file, info.span, w.name_ref(info.name).to_string())
                };
                // A parameter the compiler named (a context bound's, an anonymous `using`) is told as its class.
                let shown = if pname.contains('$') && !spelled(w, pfile, pspan, &pname) { w.name_ref(name).to_string() } else { pname };
                found.push((order, pspan.start, shown));
            }
        }
        // An object is its one instance: what it inherits from a library's class it keeps.
        if is_object(w, c) && !src.bases.get(&c).copied().unwrap_or(false) {
            found.push((order, at, w.name_ref(name).to_string()));
        }
    }
    found.sort();
    let state = found.into_iter().next().map(|(order, at, name)| (order, Kept { file: files[order], at, name }));
    // The names, wherever the sources write one: in an expression (an identifier, a selected member, an infix operator,
    // the class of a `new`), in a type, in an import's path or selector (a rename hides the name from every use).
    let mut outside: Option<(usize, Kept)> = None;
    for (order, &f) in files.iter().enumerate() {
        let ast = w.ast(f);
        let mut named: Vec<(u32, &str)> = Vec::new();
        for (i, e) in ast.exprs.iter().enumerate() {
            let span = ast.expr_spans[i];
            match e {
                Expr::Ident(n) => named.push((span.start, w.name_ref(*n))),
                // A selection's name stands at its end.
                Expr::Select(_, n) => named.push((span.end.saturating_sub(w.name_ref(*n).len() as u32), w.name_ref(*n))),
                // An infix operator: `out println text`.
                Expr::Infix(_, n, _, end) => named.push((if *end == NO_END { span.start } else { end.saturating_sub(w.name_ref(*n).len() as u32) }, w.name_ref(*n))),
                _ => {}
            }
        }
        for (i, t) in ast.tys.iter().enumerate() {
            let span = ast.ty_spans[i];
            match t {
                TyExpr::Name(n) => named.push((span.start, w.name_ref(*n))),
                TyExpr::Select(_, n) => named.push((span.end.saturating_sub(w.name_ref(*n).len() as u32), w.name_ref(*n))),
                _ => {}
            }
        }
        for imp in ast.imports.iter().chain(ast.local_imports.iter()) {
            named.extend(imp.path.iter().map(|n| (imp.span.start, w.name_ref(*n))));
            if let ImportSel::Name(n, _) = imp.sel {
                named.push((imp.span.start, w.name_ref(n)));
            }
        }
        for (at, name) in named {
            if NAMES.contains(&name) && outside.as_ref().is_none_or(|(o, k)| (order, at) < (*o, k.at)) {
                outside = Some((order, Kept { file: f, at, name: name.to_string() }));
            }
        }
    }
    // The first of the two in the files' order and the text's.
    match (state, outside) {
        (Some((a, x)), Some((b, y))) => Some(if (a, x.at) <= (b, y.at) { x } else { y }),
        (x, y) => x.or(y).map(|(_, k)| k),
    }
}

/// The names that keep a project's rules on one worker wherever its sources write one, whatever it resolves to: what
/// reaches the process's streams or the file system; what a worker's interpreter keeps a sequence of (a generator's
/// numbers, the identity hashes); and what makes a collection that carries a function (a map's default, a sorted
/// collection's ordering).
const NAMES: [&str; 25] = [
    "println", "print", "printf", "Console", "System", "Files", "Paths", "File", "PrintWriter", "FileWriter", "FileOutputStream", "printStackTrace",
    "Random", "random", "ThreadLocalRandom", "SecureRandom", "UUID", "hashCode", "##", "identityHashCode",
    "withDefault", "SortedSet", "SortedMap", "TreeSet", "TreeMap",
];

/// Whether the text spells `name` where the definition stands: the whole name at the span's start, between
/// backquotes or not.
fn spelled(w: &Worker<'_>, file: FileId, span: Span, name: &str) -> bool {
    let Some(from) = w.source(file).text.get(span.start as usize..) else { return false };
    let from = from.strip_prefix('`').unwrap_or(from);
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    from.starts_with(name) && !(name.ends_with(word) && from[name.len()..].starts_with(word))
}

/// Whether the value `s` is of a type known to hold nothing that changes.
fn value_is_known(w: &mut Worker<'_>, s: SymId, src: &Sources<'_>) -> bool {
    let t = w.sig_arc(s).ret;
    known(w, t, src, false)
}

/// The expression a value definition is given, its parentheses and ascriptions aside.
fn written(w: &Worker<'_>, s: SymId) -> Option<(FileId, crate::ast::ExprId)> {
    let info = w.syms.sym(s);
    let ast = w.ast(info.file);
    let DefKind::Val { pat: None, rhs: Some(mut e), .. } = ast.def(info.def?).kind else { return None };
    while let Expr::Parens(inner) | Expr::Typed(inner, _) = ast.expr(e) {
        e = inner;
    }
    Some((info.file, e))
}

/// Whether the member `s` is a function written as a function literal: `val f: A => B = x => ...`, in parentheses or
/// under an ascription, or as the result of a block whose statements are methods. Such a function closes over its
/// instance, whose members are read one by one, and over nothing else that outlives a call.
fn written_as_function(w: &mut Worker<'_>, s: SymId) -> bool {
    let t = w.sig_arc(s).ret;
    let function = match w.types.get(t) {
        Type::Class(c, _) => is_function_class(&w.class_path(c)),
        Type::Lambda(..) | Type::Poly(..) => true,
        _ => false,
    };
    let Some((file, mut e)) = written(w, s) else { return false };
    let ast = w.ast(file);
    loop {
        match ast.expr(e) {
            Expr::Lambda(..) | Expr::PolyLambda(..) => return function,
            Expr::Parens(inner) | Expr::Typed(inner, _) => e = inner,
            Expr::Block(stmts) => {
                let stmts = ast.stmt_list(stmts);
                let Some((Stmt::Expr(last), before)) = stmts.split_last() else { return false };
                if !before.iter().all(|st| matches!(st, Stmt::Def(d) if matches!(ast.def(*d).kind, DefKind::Fun(_)))) {
                    return false;
                }
                e = *last;
            }
            _ => return false,
        }
    }
}

/// Whether the value `s` is a `SymbolMatcher` written as a call of the API's own makers, `SymbolMatcher.exact(...)` or
/// `SymbolMatcher.normalized(...)`, or as such calls and other definitions joined by `+`: what they make closes over
/// their strings alone.
/// One made any other way (a class of the sources, a function literal, a method's result) may hold anything; and
/// the name is the API's only where the sources define and import nothing else called `SymbolMatcher`.
fn written_as_matcher(w: &mut Worker<'_>, s: SymId, src: &Sources<'_>) -> bool {
    let t = w.sig_arc(s).ret;
    let matcher = match w.types.get(t) {
        Type::Class(c, _) => w.class_path(c) == "teq.fix.SymbolMatcher",
        _ => false,
    };
    let Some((file, e)) = written(w, s) else { return false };
    matcher && made_by_the_api(w, w.ast(file), e) && !src.own_matcher
}

fn made_by_the_api(w: &Worker<'_>, ast: &crate::ast::Ast, e: crate::ast::ExprId) -> bool {
    match ast.expr(e) {
        Expr::Parens(inner) => made_by_the_api(w, ast, inner),
        // An operand that is a name is another definition, read where it stands.
        Expr::Infix(l, op, r, _) => w.name_ref(op) == "+" && [l, r].into_iter().all(|x| matches!(ast.expr(x), Expr::Ident(_)) || made_by_the_api(w, ast, x)),
        Expr::Apply(f, _) => match ast.expr(f) {
            Expr::Select(q, n) if matches!(w.name_ref(n), "exact" | "normalized") => match ast.expr(q) {
                Expr::Ident(o) | Expr::Select(_, o) => w.name_ref(o) == "SymbolMatcher",
                _ => false,
            },
            _ => false,
        },
        _ => false,
    }
}

fn is_function_class(path: &str) -> bool {
    path.starts_with("scala.Function") || path.starts_with("scala.ContextFunction") || path == "scala.PartialFunction"
}

/// Whether a value of type `t` is known to hold nothing that changes: the primitives and `String`, the immutable
/// collections with `Option`, `Either` and the tuples over such types, a regular expression, a closed type of the
/// rules' own API, and a case class, an enum or an object of the sources whose members are such values. A function is
/// not: what it closed over is not in its type (`written_as_function` reads the one shape that says); nor is a type
/// of the API that a rule's sources may extend (a rule, a diagnostic, a matcher), a type parameter or a wildcard, of
/// which the type says nothing. `result` is for a method's result, where the question is what can carry a function
/// out of the call to something that keeps it: a type parameter cannot (the method cannot make one), nor can an array
/// or a mutable collection of plain data, nor a diagnostic, which goes where a `Patch` goes.
fn known(w: &mut Worker<'_>, t: TypeId, src: &Sources<'_>, result: bool) -> bool {
    let mut waits = Vec::new();
    says(w, t, src, result, &[], &mut waits) && waits.iter().all(|c| !src.unknown[result as usize].contains_key(c))
}

/// What the type `t` says by itself, the data classes of the sources it names left to `waits`: their answer is
/// `settle`'s, one a class. `own` are the type parameters of the data class whose member's type `t` is: what stands
/// for one is asked where the class is applied, with its arguments.
fn says(w: &mut Worker<'_>, t: TypeId, src: &Sources<'_>, result: bool, own: &[TParamId], waits: &mut Vec<ClassId>) -> bool {
    match w.types.get(t) {
        Type::Nothing | Type::Lit(_) => true,
        Type::Param(p) => result || own.contains(&p),
        Type::Annotated(u) => says(w, u, src, result, own, waits),
        Type::Union(a, b, _) | Type::Inter(a, b) => says(w, a, src, result, own, waits) && says(w, b, src, result, own, waits),
        Type::Nested(_, inner) => says(w, inner, src, result, own, waits),
        Type::Class(c, args) => {
            let args: Vec<TypeId> = w.types.items(args).to_vec();
            class_says(w, c, src, result, waits) && args.into_iter().all(|a| says(w, a, src, result, own, waits))
        }
        _ => false,
    }
}

/// Whether the class `c` of the sources is data whose members decide: a case class, an enum or an enum's case that
/// no other class of the sources extends and that inherits nothing that holds anything.
fn is_data(w: &Worker<'_>, c: ClassId, src: &Sources<'_>) -> bool {
    let info = w.syms.class(c);
    (info.mods & crate::ast::mods::CASE != 0 || matches!(info.kind, ClassKind::Enum | ClassKind::EnumCase)) && !src.extended.contains(&c) && src.bases.get(&c).copied().unwrap_or(false)
}

/// Whether the class `c` of the sources is data and nothing else: its values hold what their members hold and are
/// kept only by what names their type. Not so a class the rule API stands behind (a rule, a diagnostic or a matcher
/// written as a case class), whose instance the run itself may keep.
fn data_apart(w: &Worker<'_>, c: ClassId, src: &Sources<'_>) -> bool {
    src.files.contains(&w.syms.class(c).file) && !is_object(w, c) && is_data(w, c, src) && src.apart.get(&c).copied().unwrap_or(false)
}

fn class_says(w: &mut Worker<'_>, c: ClassId, src: &Sources<'_>, result: bool, waits: &mut Vec<ClassId>) -> bool {
    let (file, kind) = {
        let info = w.syms.class(c);
        (info.file, info.kind)
    };
    if src.files.contains(&file) {
        // An object is its one instance (one an enclosing instance where it is nested in a class), whose members are
        // read where they stand, each for itself: what it inherits is all its type adds.
        if is_object(w, c) {
            return src.bases.get(&c).copied().unwrap_or(false);
        }
        // A case class, an enum or an enum's case of the sources: its own members (`settle`).
        if !is_data(w, c, src) {
            return false;
        }
        waits.push(c);
        return true;
    }
    let path = w.class_path(c);
    if kind == ClassKind::Builtin {
        // The primitives, `String`, `Unit` and `Null`; an array's cells change (as a result it carries what its cells
        // do), and `AnyRef` or `AnyVal` says nothing of what the value is.
        return match path.as_str() {
            "scala.Array" => result,
            "scala.AnyRef" | "scala.AnyVal" => false,
            _ => true,
        };
    }
    // The immutable collections by their names in teq's std, which the rules' program is typed under (its `List`,
    // `Vector`, `Map`, `Set` and `Seq` are `package scala`'s: std/collections.scala, iterable.scala, maps.scala), and
    // what else of the library and the JDK holds nothing that changes. `scala.Iterable` is not among them: the std's
    // mutable collections extend it; nor is a builder of the immutable package, nor a sorted collection, which carries
    // its ordering, nor `scala.Tuple`, which says nothing of what the tuple holds.
    const EXACT: [&str; 31] = [
        "scala.Option", "scala.Some", "scala.None", "scala.Either", "scala.Left", "scala.Right", "scala.util.Either", "scala.util.Left", "scala.util.Right",
        "scala.List", "scala.::", "scala.Nil", "scala.Vector", "scala.Range", "scala.LazyList", "scala.Map", "scala.Set", "scala.Seq", "scala.IndexedSeq",
        "scala.Symbol", "scala.math.BigInt", "scala.math.BigDecimal", "scala.util.matching.Regex", "java.util.regex.Pattern", "java.lang.String",
        "java.lang.Integer", "java.lang.Long", "java.lang.Double", "java.lang.Character", "java.lang.Boolean", "scala.Unit",
    ];
    // The API's types a rule's sources may extend: a value of one may be an instance of a class of theirs.
    const OPEN: [&str; 5] = ["teq.fix.Rule", "teq.fix.SyntacticRule", "teq.fix.SemanticRule", "teq.fix.Diagnostic", "teq.fix.SymbolMatcher"];
    EXACT.contains(&path.as_str())
        || (path.starts_with("scala.collection.immutable.") && !path.ends_with("Builder") && !path.contains("Sorted") && !path.contains("Tree"))
        || path.strip_prefix("scala.Tuple").is_some_and(|arity| !arity.is_empty() && arity.bytes().all(|b| b.is_ascii_digit()))
        || (path.starts_with("teq.fix.") && !OPEN.contains(&path.as_str()))
        || (result && (path.starts_with("scala.collection.mutable.") || path == "teq.fix.Diagnostic"))
}

/// The data classes of the sources that are not known to hold nothing, for both questions: one with a `var`, or with
/// a value whose type says so by itself, and then every one that holds a value of such a class, until none is added.
/// A class is read once; classes that only name one another hold nothing.
fn settle(w: &mut Worker<'_>, classes: &[ClassId], src: &mut Sources<'_>) {
    for result in [false, true] {
        let mut unknown: FxMap<ClassId, ()> = FxMap::default();
        let mut held_by: FxMap<ClassId, Vec<ClassId>> = FxMap::default();
        let mut found: Vec<ClassId> = Vec::new();
        for &c in classes {
            if is_object(w, c) || !is_data(w, c, src) {
                continue;
            }
            let (members, own): (Vec<SymId>, Vec<TParamId>) = (w.syms.class(c).member_order.clone(), w.syms.class(c).tparams.clone());
            let mut waits = Vec::new();
            let holds = members.into_iter().all(|m| match w.syms.sym(m).kind {
                SymKind::Var => false,
                SymKind::Val => {
                    let t = w.sig_arc(m).ret;
                    says(w, t, src, result, &own, &mut waits)
                }
                _ => true,
            });
            if !holds {
                unknown.insert(c, ());
                found.push(c);
            }
            for d in waits {
                held_by.entry(d).or_default().push(c);
            }
        }
        while let Some(d) = found.pop() {
            for &c in held_by.get(&d).map_or(&[][..], |v| v.as_slice()) {
                if unknown.insert(c, ()).is_none() {
                    found.push(c);
                }
            }
        }
        src.unknown[result as usize] = unknown;
    }
}

/// Whether the class `c` is an object's: one at the top level or in an object, or one nested in a class, a trait or a
/// block, which is a class of its own with a value of the enclosing scope holding its instance (an enum's or a case
/// class's companion made there among them).
fn is_object(w: &Worker<'_>, c: ClassId) -> bool {
    let info = w.syms.class(c);
    info.kind == ClassKind::Object || info.inner_object.is_some() || info.local_module.is_some()
}

/// Whether what the class `c` of the sources inherits holds nothing, kept in `src.bases`, one answer a class: each
/// parent it writes is a class of the sources that inherits nothing either (its own members are read where they
/// stand), or one of the library's bases that hold nothing. The parents the compiler gives a case class, an object
/// or an enum are of those. `apart` asks the same with the rule API's types and the functions taken out of the
/// bases (`src.apart`): whether the class stands apart from what the run or a function's keeper holds.
fn bases_of(w: &mut Worker<'_>, c: ClassId, src: &mut Sources<'_>, apart: bool) -> bool {
    if let Some(&known) = if apart { src.apart.get(&c) } else { src.bases.get(&c) } {
        return known;
    }
    // A class under way says nothing against itself.
    if apart { src.apart.insert(c, true) } else { src.bases.insert(c, true) };
    let (parents, given) = (w.syms.class(c).parents.clone(), w.syms.class(c).desugared_parents as usize);
    let written = parents.len().saturating_sub(given);
    let mut holds = true;
    for p in parents.into_iter().take(written) {
        holds = holds
            && match w.types.get(p) {
                Type::Class(b, _) => {
                    if src.files.contains(&w.syms.class(b).file) {
                        bases_of(w, b, src, apart)
                    } else {
                        let path = w.class_path(b);
                        const BASES: [&str; 18] = [
                            "scala.AnyRef", "scala.Any", "scala.AnyVal", "java.lang.Object", "scala.Product", "scala.Equals", "scala.Serializable", "java.io.Serializable", "scala.reflect.Enum",
                            "scala.math.Ordering", "scala.Ordering", "scala.math.Ordered", "scala.Ordered", "java.lang.Comparable", "scala.CanEqual", "scala.Conversion",
                            "scala.annotation.StaticAnnotation", "scala.annotation.Annotation",
                        ];
                        BASES.contains(&path.as_str()) || path.starts_with("scala.deriving.Mirror") || (!apart && (path.starts_with("teq.fix.") || is_function_class(&path)))
                    }
                }
                _ => false,
            };
    }
    if apart { src.apart.insert(c, holds) } else { src.bases.insert(c, holds) };
    holds
}
