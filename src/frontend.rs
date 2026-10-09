//! Lexing and parsing of all source files, in parallel when the input is large enough.
//! Workers use their own interner; afterwards the names in each AST are remapped to the
//! global interner, which is cheap because AST nodes live in flat arrays.
//!
//! The std files of a build have their slots in the file table from the start, in the order of
//! `main.rs`'s lists, but only those the program's text names are parsed with it
//! (`select_std`, from the names the program's files use); the rest get empty ASTs and are
//! parsed when a lookup first asks for one of their names (`typer/stdlib.rs`).

use crate::ast::*;
use crate::intern::{Interner, Name};
use crate::source::{Diagnostics, FileId, SourceFile, Span};
use crate::stdindex::{StdFileIndex, TERM, TYPE};
use crate::typer::stdlib::Layer;
use crate::{lexer, parser};
use std::sync::atomic::{AtomicUsize, Ordering};

const PARALLEL_THRESHOLD_BYTES: usize = 256 * 1024;

/// The typer's threshold: a program of fewer bytes of source types its bodies on one worker.
/// The fork first pays on the corpus's code from 166 KB on the reference machine
/// and 213 KB on a Linux test machine, on macro-heavy files from 6 KB, and loses 3 to 12 ms where it
/// does not pay; 512 KiB leaves the budget's feature programs under it serial. It moves down with
/// the fork's fixed cost.
const TYPER_THRESHOLD_BYTES: usize = 512 * 1024;

/// The most workers the typer's automatic count gives: from 8 to 12
/// the applications gain 8 to 9% and the corpus 12 to 18%, where programs of few large units lose
/// 7 to 29% (the extra workers' fixed cost, and on a machine of 12 performance cores under other
/// load a lock-bound program's holders on its efficiency cores); 12 once the fork path's base cost
/// takes that loss.
const TYPER_CAP: usize = 8;

/// The parser threads for the std files the program names, parsed after the program's.
pub const STD_WORKERS: usize = 4;

type FileErrors = Vec<(Span, String)>;

/// A std file of the build before it is parsed.
pub struct StdInput {
    pub index: &'static StdFileIndex,
    pub text: &'static str,
    pub layer: Layer,
}

/// The files the std entered always: what the language's own constructs need without naming
/// it (tuples, `Product`, `Conversion`, the evidence classes, `scala.reflect.Enum`, the
/// members of the builtin classes), what every program calls (`println`), and the JVM runtime
/// when the target is the JVM, which its backend calls by name.
const STD_CORE: [&str; 5] = ["<std>/core.scala", "<std>/prelude.scala", "<std>/evidence.scala", "<std>/enum.scala", "<std>/jvm.scala"];

/// The builtin classes of package `scala`, whose names never reach a same-named definition of
/// another package by a bare reference.
const BUILTIN_NAMES: [&str; 16] = [
    "Int", "Long", "Double", "Byte", "Short", "Float", "Boolean", "String", "Char", "Unit", "Array", "Null", "AnyRef", "AnyVal", "Any", "Nothing",
];

/// What the program's files name, as a bitset over the interner, and the constructs whose
/// typing needs a std class the text does not name.
#[derive(Default)]
pub struct UsedNames {
    /// Names used as terms (bit `TERM`) or types (bit `TYPE`), by name.
    pub names: Vec<u8>,
    pub varargs: bool,
    pub interpolation: bool,
    pub exceptions: bool,
    pub named_tuples: bool,
    pub quotes: bool,
    /// The program defines a package of the Scala.js layer itself: the layer stays out.
    pub defines_scalajs: bool,
    /// The top-level definitions of the program, as (package path, name): a std definition of
    /// that qualified name is shadowed, so no lookup ever asks the std for it.
    pub defined: Vec<(String, String)>,
}

impl UsedNames {
    fn mark(&mut self, n: Name, kind: u8) {
        let i = n.0 as usize;
        if self.names.len() <= i {
            self.names.resize(i + 1, 0);
        }
        self.names[i] |= kind;
    }

    /// Whether the program uses `n` as one of `kind` (`TYPE`, `TERM` or both).
    pub fn used(&self, n: Name, kind: u8) -> bool {
        self.names.get(n.0 as usize).map_or(false, |k| k & kind != 0)
    }

    fn merge(&mut self, other: &UsedNames, map: &[Name]) {
        for (i, &k) in other.names.iter().enumerate() {
            if k != 0 {
                self.mark(map[i], k);
            }
        }
        self.varargs |= other.varargs;
        self.interpolation |= other.interpolation;
        self.exceptions |= other.exceptions;
        self.named_tuples |= other.named_tuples;
        self.quotes |= other.quotes;
        self.defines_scalajs |= other.defines_scalajs;
        self.defined.extend(other.defined.iter().cloned());
    }
}

/// Records the names `ast` refers to: identifiers and selections in expressions, types, imports
/// and annotations, and the segments of import paths.
pub fn note_used_names(ast: &Ast, used: &mut UsedNames, interner: &Interner) {
    for e in &ast.exprs {
        match e {
            Expr::Ident(n) | Expr::Select(_, n) | Expr::Infix(_, n, _) | Expr::Prefix(n, _) | Expr::NamedArg(n, _) => used.mark(*n, TERM),
            Expr::Interp(n, _, _) => {
                used.mark(*n, TERM);
                used.interpolation = true;
            }
            Expr::Throw(_) | Expr::Try(_) => used.exceptions = true,
            Expr::NamedTuple(..) => used.named_tuples = true,
            Expr::Quote(_) | Expr::QuoteType(_) | Expr::Splice(_) | Expr::SplicePat(_) => used.quotes = true,
            _ => {}
        }
    }
    for t in &ast.tys {
        match t {
            TyExpr::Name(n) | TyExpr::Project(_, n) => used.mark(*n, TYPE),
            // The prefix of a type selection is a term path, and its last segment a type.
            TyExpr::Select(q, n) => {
                used.mark(*n, TYPE);
                mark_type_prefix(ast, *q, used);
            }
            TyExpr::Repeated(_) => used.varargs = true,
            TyExpr::NamedTuple(..) => used.named_tuples = true,
            _ => {}
        }
    }
    for p in &ast.pats {
        match p {
            Pat::Rest(_) => used.varargs = true,
            Pat::Quote(_) | Pat::QuoteType(_) => used.quotes = true,
            _ => {}
        }
    }
    let clauses = ast.imports.iter().chain(&ast.local_imports).chain(&ast.exports).chain(&ast.top_exports);
    for imp in clauses {
        for &n in &imp.path {
            used.mark(n, TERM);
        }
        if let ImportSel::Name(n, _) = imp.sel {
            used.mark(n, TYPE | TERM);
        }
    }
    for a in ast.param_annots.iter().chain(ast.defs.iter().flat_map(|d| d.annots.iter())) {
        used.mark(a.name, TYPE | TERM);
    }
    let pkg: String = ast.package.iter().map(|&n| interner.get(n)).collect::<Vec<_>>().join(".");
    for &d in &ast.top_level {
        used.defined.push((pkg.clone(), interner.get(ast.def(d).name).to_string()));
    }
    for exp in &ast.top_exports {
        if let ImportSel::Name(n, alias) = exp.sel {
            used.defined.push((pkg.clone(), interner.get(alias.unwrap_or(n)).to_string()));
        }
    }
    if ast.package.len() >= 2 && interner.get(ast.package[1]) == "scalajs" {
        let root = interner.get(ast.package[0]);
        if root == "scala" || root == "org" {
            used.defines_scalajs = true;
        }
    }
    used.quotes |= ast.has_quotes;
}

fn mark_type_prefix(ast: &Ast, t: TyExprId, used: &mut UsedNames) {
    match ast.ty(t) {
        TyExpr::Name(n) => used.mark(n, TERM),
        TyExpr::Select(q, n) => {
            used.mark(n, TERM);
            mark_type_prefix(ast, q, used);
        }
        _ => {}
    }
}

/// The file's AST first, then one per `package p:` block. Where the grammar of an unclosed
/// parenthesis or bracket fails at the first token of a line that its owning region holds
/// (`LexResult::boundaries`), the file is lexed and parsed again with the delimiters closed
/// before that line, at most `MAX_RELEXES` times.
pub fn parse_one(text: &str, interner: &Interner, syntax: parser::Syntax) -> (Vec<Ast>, FileErrors) {
    let mut closed: Vec<u32> = Vec::new();
    loop {
        let lexed = lexer::lex_closing(text, interner, &closed);
        let parsed = parser::parse(text, &lexed, &closed, interner, syntax);
        match parsed.boundary {
            Some(b) if closed.len() < lexer::MAX_RELEXES && !closed.contains(&b) => {
                closed.push(b);
                closed.sort_unstable();
            }
            _ => {
                let mut errors = lexed.errors;
                errors.extend(parsed.errors);
                return (parsed.asts, errors);
            }
        }
    }
}

/// Whether the program is large enough for the parser threads.
pub fn parallel(files: &[SourceFile]) -> bool {
    threads_by_size(files) >= 2
}

/// The typer's workers a build asked for: `--threads n` (`requested`, 0 when not given), else
/// `TEQ_THREADS`, which stands in for it where the suites run the binary; as given, past every
/// automatic bound.
pub fn requested_threads(requested: usize) -> Option<usize> {
    if requested >= 1 {
        return Some(requested);
    }
    std::env::var("TEQ_THREADS").ok().and_then(|n| n.parse::<usize>().ok()).filter(|&n| n >= 1)
}

/// The typer's workers of a build that asks for none, by the bytes of its program's sources, each
/// file once, and the bytes of its largest unit of work (`largest_unit_bytes`): one under
/// `TYPER_THRESHOLD_BYTES` or where one unit holds more than half the bytes, which
/// one worker types whatever the count; else the least of the cores (`TEQ_WORKERS` bounding them),
/// `TYPER_CAP` and the memory's (`memory::workers`).
pub fn automatic_threads(program_bytes: usize, largest_unit: usize) -> usize {
    if program_bytes < typer_threshold() || 2 * largest_unit > program_bytes {
        return 1;
    }
    crate::workers().min(TYPER_CAP).min(crate::memory::workers())
}

/// `TYPER_THRESHOLD_BYTES`, or the bytes `TEQ_TYPER_THRESHOLD` gives, a diagnostic: the automatic count
/// at another threshold, which bench/ptyper-sweep.py's automatic arms measure.
fn typer_threshold() -> usize {
    std::env::var("TEQ_TYPER_THRESHOLD").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(TYPER_THRESHOLD_BYTES)
}

/// The bytes of the program's largest unit of the typer's parallel work: a top-level definition of
/// a file or of one of its `package p:` blocks (`check::Queue`'s item), from the line its
/// annotations and modifiers start on to the next one's in the file, or to the file's end.
/// `blocks_of` is the layout's, by original file.
pub fn largest_unit_bytes(files: &[SourceFile], asts: &[Ast], blocks_of: &[Vec<usize>]) -> usize {
    let mut largest = 0;
    for (file, blocks) in blocks_of.iter().enumerate().filter(|&(i, _)| !files[i].is_std) {
        let text = &files[file].text;
        let line_of = |at: usize| text[..at.min(text.len())].rfind('\n').map_or(0, |n| n + 1);
        let mut starts: Vec<usize> = std::iter::once(&file)
            .chain(blocks)
            .flat_map(|&a| asts[a].top_level.iter().map(move |&d| asts[a].def_range(d).start as usize))
            .map(line_of)
            .collect();
        starts.sort_unstable();
        starts.push(files[file].text.len());
        largest = starts.windows(2).map(|w| w[1] - w[0]).fold(largest, usize::max);
    }
    largest
}

/// The parser's rule for the worker count: one per core for a large program.
pub fn threads_by_size(files: &[SourceFile]) -> usize {
    let total: usize = files.iter().filter(|f| !f.is_std).map(|f| f.text.len()).sum();
    if total >= PARALLEL_THRESHOLD_BYTES {
        crate::workers()
    } else {
        1
    }
}

/// Parses the files whose slot is empty among `todo` and fills it with the file's AST followed
/// by one per `package p:` block; a slot already filled is a file parsed earlier and left
/// alone. With `used`, the names the files refer to are recorded.
pub fn parse_files(
    files: &[SourceFile],
    todo: impl Iterator<Item = usize>,
    slots: &mut [Option<Vec<Ast>>],
    interner: &mut Interner,
    diags: &mut Diagnostics,
    mut used: Option<&mut UsedNames>,
    parallel: bool,
    max_workers: usize,
    syntax: parser::Syntax,
) {
    let todo: Vec<usize> = todo.filter(|&i| slots[i].is_none()).collect();
    let workers = crate::workers().min(todo.len()).min(max_workers);
    if !parallel || workers < 2 {
        for &i in &todo {
            let (asts, errors) = parse_one(&files[i].text, interner, syntax);
            for (span, msg) in errors {
                diags.error(FileId(i as u32), span, msg);
            }
            if let Some(used) = used.as_deref_mut() {
                for ast in &asts {
                    note_used_names(ast, used, interner);
                }
            }
            slots[i] = Some(asts);
        }
    } else {
        // Largest files first so that the tail of the work is evenly spread.
        let mut order = todo;
        order.sort_by_key(|&i| std::cmp::Reverse(files[i].text.len()));
        let next = AtomicUsize::new(0);
        let note = used.is_some();
        let results: Vec<(Interner, UsedNames, Vec<(usize, Vec<Ast>, FileErrors)>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|_| {
                    crate::alloc::spawn_in(scope, || {
                        let mut local = Interner::new();
                        let mut local_used = UsedNames::default();
                        let mut done = Vec::new();
                        loop {
                            let slot = next.fetch_add(1, Ordering::Relaxed);
                            let Some(&i) = order.get(slot) else { break };
                            let (asts, errors) = parse_one(&files[i].text, &mut local, syntax);
                            if note {
                                for ast in &asts {
                                    note_used_names(ast, &mut local_used, &local);
                                }
                            }
                            done.push((i, asts, errors));
                        }
                        (local, local_used, done)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().expect("parser thread panicked")).collect()
        });
        for (local, local_used, done) in results {
            let map: Vec<Name> = (0..local.len()).map(|n| interner.intern(local.get(Name(n as u32)))).collect();
            if let Some(used) = used.as_deref_mut() {
                used.merge(&local_used, &map);
            }
            for (i, mut asts, errors) in done {
                for ast in &mut asts {
                    remap_names(ast, &map);
                }
                for (span, msg) in errors {
                    diags.error(FileId(i as u32), span, msg);
                }
                slots[i] = Some(asts);
            }
        }
    }
}

/// Which std files to parse with the program: the core, and every file defining a name the
/// program uses in a package a bare name or a path can reach. A builtin's name (`Long`,
/// `String`) never reaches `java.lang`'s or the facades' class of that name. The Scala.js layer
/// counts when it is unlocked from the start: a Scala.js jar on the classpath, or the program
/// naming one of its packages without defining it.
pub fn select_std(inputs: &[StdInput], used: &UsedNames, interner: &Interner, scalajs_unlocked: bool) -> Vec<bool> {
    let mut seeds: Vec<&str> = Vec::new();
    if used.varargs {
        seeds.push("Seq");
    }
    if used.interpolation {
        seeds.push("StringContext");
    }
    if used.exceptions {
        seeds.extend(["Throwable", "JavaScriptException"]);
    }
    if used.named_tuples {
        seeds.push("NamedTuple");
    }
    if used.quotes {
        seeds.extend(["Quotes", "Expr", "Type"]);
    }
    let named = |name: &str, kind: u8| interner.lookup(name).map_or(false, |n| used.used(n, kind)) || seeds.contains(&name);
    // A package a bare name or a path can reach: `scala` and `java.lang` always, any other
    // when every segment of its path is a term the program names (an import, a qualified
    // reference).
    let reachable = |pkg: &str| pkg == "scala" || pkg == "java.lang" || pkg.split('.').all(|seg| named(seg, TERM));
    let defined_by_program = |pkg: &str, name: &str| used.defined.iter().any(|(p, n)| p == pkg && n == name);
    inputs
        .iter()
        .map(|input| {
            if input.layer == Layer::ScalaJs && !scalajs_unlocked {
                return false;
            }
            if STD_CORE.contains(&input.index.path) {
                return true;
            }
            input.index.defines.iter().any(|&(pkg, name, kind)| {
                reachable(pkg) && (pkg == "scala" || !BUILTIN_NAMES.contains(&name)) && named(name, kind) && !defined_by_program(pkg, name)
            })
        })
        .collect()
}

/// Whether the program names a package of the Scala.js layer (`scala.scalajs`, `org.scalajs`)
/// without defining one itself.
pub fn names_scalajs(used: &UsedNames, interner: &Interner) -> bool {
    !used.defines_scalajs && interner.lookup("scalajs").map_or(false, |n| used.used(n, TERM))
}

/// Lays the parsed files out by file id: the files in their order, then every package block,
/// which gets a file entry sharing the source of its file. A std file left unparsed gets an
/// empty AST and one per block its index counts, so that the ids are the same whatever was
/// parsed. Also returns, per file, the ids of its blocks.
pub fn lay_out(files: &mut Vec<SourceFile>, slots: Vec<Option<Vec<Ast>>>, std: &[StdInput]) -> (Vec<Ast>, Vec<Vec<usize>>) {
    let mut asts = Vec::with_capacity(files.len());
    let mut blocks = Vec::new();
    let mut blocks_of = vec![Vec::new(); files.len()];
    for (i, parsed) in slots.into_iter().enumerate() {
        let mut parsed = match parsed {
            Some(parsed) => parsed,
            None => {
                let n = std.get(i).map_or(0, |s| s.index.blocks as usize);
                (0..=n).map(|_| Ast::new(0)).collect()
            }
        };
        for block in parsed.drain(1..) {
            blocks.push((i, block));
        }
        asts.extend(parsed);
    }
    for (i, block) in blocks {
        let mut block_file = files[i].copy();
        block_file.key.push_str(&format!("#{}", blocks_of[i].len() + 1));
        files.push(block_file);
        blocks_of[i].push(asts.len());
        asts.push(block);
    }
    (asts, blocks_of)
}

/// Whether a program file quotes or splices anywhere.
pub fn uses_quotes(asts: &Asts, files: &[SourceFile]) -> bool {
    (0..asts.len()).any(|i| files.get(i).map_or(false, |f| !f.is_std) && asts[i].has_quotes)
}

fn remap_names(ast: &mut Ast, map: &[Name]) {
    let m = |n: &mut Name| *n = map[n.0 as usize];
    for e in &mut ast.exprs {
        match e {
            Expr::Ident(n) | Expr::Select(_, n) | Expr::NamedArg(n, _) | Expr::Infix(_, n, _)
            | Expr::Prefix(n, _) | Expr::Interp(n, _, _) | Expr::Super(n) => m(n),
            _ => {}
        }
    }
    for p in &mut ast.pats {
        if let Pat::Bind(n, _) = p {
            m(n);
        }
    }
    for t in &mut ast.tys {
        if let TyExpr::Name(n) | TyExpr::Select(_, n) | TyExpr::Project(_, n) | TyExpr::TypeVar(n) = t {
            m(n);
        }
    }
    ast.name_lists.iter_mut().for_each(m);
    ast.param_annots.iter_mut().for_each(|a| m(&mut a.name));
    ast.package.iter_mut().for_each(m);
    ast.access_scopes.iter_mut().for_each(|(_, n)| m(n));
    for lp in &mut ast.lambda_params {
        m(&mut lp.name);
    }
    let clauses = ast.imports.iter_mut().chain(&mut ast.local_imports).chain(&mut ast.exports);
    for imp in clauses.chain(&mut ast.top_exports) {
        imp.path.iter_mut().for_each(m);
        if let ImportSel::Name(n, alias) = &mut imp.sel {
            m(n);
            if let Some(a) = alias {
                m(a);
            }
        }
    }
    let remap_tparams = |tps: &mut Vec<TypeParam>| {
        for tp in tps {
            tp.name = map[tp.name.0 as usize];
            tp.evidence_names.iter_mut().for_each(|n| *n = map[n.0 as usize]);
            tp.annots.iter_mut().for_each(|a| a.name = map[a.name.0 as usize]);
        }
    };
    let remap_clauses = |clauses: &mut Vec<ParamClause>| {
        for c in clauses {
            for p in &mut c.params {
                p.name = map[p.name.0 as usize];
            }
        }
    };
    for d in &mut ast.defs {
        m(&mut d.name);
        for a in &mut d.annots {
            m(&mut a.name);
        }
        match &mut d.kind {
            DefKind::Val { .. } => {}
            DefKind::Fun(f) => {
                remap_tparams(&mut f.tparams);
                remap_clauses(&mut f.clauses);
            }
            DefKind::Class(c) => {
                remap_tparams(&mut c.tparams);
                remap_clauses(&mut c.clauses);
            }
            DefKind::TypeAlias { tparams, .. } => remap_tparams(tparams),
            DefKind::Given(g) => {
                remap_tparams(&mut g.tparams);
                remap_clauses(&mut g.clauses);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(texts: &[&str]) -> usize {
        let interner = Interner::new();
        let mut files = Vec::new();
        let mut asts = Vec::new();
        let mut blocks_of = Vec::new();
        for (i, text) in texts.iter().enumerate() {
            files.push(SourceFile { path: format!("{}.scala", i), text: text.to_string(), is_std: false, key: format!("{}.scala", i) });
            let (mut parsed, errors) = parse_one(text, &interner, parser::Syntax::default());
            assert!(errors.is_empty(), "{:?}", errors);
            asts.push(parsed.remove(0));
            blocks_of.push(Vec::new());
            for block in parsed {
                blocks_of[i].push(asts.len());
                asts.push(block);
            }
        }
        largest_unit_bytes(&files, &asts, &blocks_of)
    }

    #[test]
    fn the_largest_unit_runs_to_the_next_definition() {
        let a = "object A:\n  def f = 1\n";
        let b = "object B:\n  def g = 2\n  def h = 3\n";
        assert_eq!(units(&[&format!("{a}{b}")]), b.len());
        assert_eq!(units(&[a, b]), b.len());
        assert_eq!(units(&[&format!("import scala.math.*\n{a}")]), a.len());
        assert_eq!(units(&["package p\n"]), 0);
        let annotated = "@deprecated(\"old\", \"1\")\nobject B:\n  def g = 2\n";
        assert_eq!(units(&[&format!("{a}{annotated}")]), annotated.len());
    }

    #[test]
    fn a_package_block_holds_units_of_its_own() {
        let text = "package p:\n  object A:\n    def f = 1\npackage q:\n  object B:\n    def g = 2\n";
        let (first, second) = (text.find("  object A").unwrap(), text.find("  object B").unwrap());
        assert_eq!(units(&[text]), second - first);
    }
}
