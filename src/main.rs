// `names::well_known!` recurses once per name.
#![recursion_limit = "512"]

#[cfg_attr(feature = "system-alloc", allow(dead_code))]
mod alloc;
mod argfile;
mod arena;
mod ast;
mod classfile;
mod classpath;
mod complete;
mod crew;
mod dialect;
mod emit;
mod frontend;
mod held;
mod index;
mod intern;
mod jarcache;
mod interp;
mod jvm;
mod lexer;
mod lsp;
mod measure;
mod memory;
mod names;
mod parser;
mod planted;
mod products;
mod report;
mod scala2;
mod shake;
mod shape;
mod shared;
mod source;
mod stdindex;
mod symbols;
mod task;
mod tasty;
mod text;
mod tir;
mod token;
mod typer;
mod types;
mod watch;
mod write;
mod zip;

use ast::Asts;
use source::{Diagnostics, SourceFile, Sources};

#[cfg(not(feature = "system-alloc"))]
#[global_allocator]
static GLOBAL: alloc::TeqAlloc = alloc::TeqAlloc;
use std::time::{Duration, Instant};

/// The threads a phase spreads its work over: the machine's, or at most `TEQ_WORKERS`.
pub(crate) fn workers() -> usize {
    static CAP: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let cap = *CAP.get_or_init(|| std::env::var("TEQ_WORKERS").ok().and_then(|v| v.parse().ok()).filter(|&n| n > 0).unwrap_or(usize::MAX));
    std::thread::available_parallelism().map_or(1, |n| n.get()).min(cap)
}

/// The std of a mode; on the JVM, whose one mode is scala-library's, without the `java.*`
/// classes written for the interpreter's file reading, which the JDK's own take the place of
/// there.
pub(crate) fn std_sources(std: StdMode, jvm: bool) -> Vec<(&'static str, &'static str)> {
    debug_assert!(!jvm || std == StdMode::ScalaLibrary, "a JVM build links against scala-library");
    std.sources()
        .iter()
        .copied()
        .filter(|(path, _)| !jvm || !JS_ONLY_LAYER.contains(path))
        .filter(|(path, _)| !jvm || !NON_JVM_LAYER.contains(path))
        .filter(|(path, _)| !jvm || !LINKED_LAYER.contains(path))
        .collect()
}

/// The buffer of JavaScript and the interpreter, the array itself; the JVM's is the class of
/// std/jvm_buffer.scala.
const NON_JVM_LAYER: [&str; 1] = ["<std>/buffer.scala"];

/// The layer's files that the JVM takes from scala-library's bytecode instead: its `IArray`
/// makes arrays without a `ClassTag`, which JVM arrays need, and its `ValueOf` is a value class,
/// which descriptors erase to the value.
const LINKED_LAYER: [&str; 2] = ["<std>/iarray.scala", "<std>/value_of.scala"];

/// Every std file of a build, in the order of their slots in the file table: the mode's files,
/// the JVM runtime under `--target jvm`, the quoted layer and the Scala.js facades. Which of
/// them are parsed and entered is decided by name (`frontend::select_std`, `typer/stdlib.rs`).
/// The std's DOM shapes serve a program without scalajs-dom: a jar on the class path that holds
/// the TASTy of `org.scalajs.dom` is its definition, and the std's files of the package are left
/// out of the build.
pub(crate) fn std_layout(opts: &Options, cp: Option<&classpath::Classpath>) -> Vec<frontend::StdInput> {
    let jvm: &[(&str, &str)] = if opts.jvm { JVM_STD_SOURCES } else { &[] };
    let base = std_sources(opts.std, opts.jvm).into_iter().chain(jvm.iter().copied()).chain(QUOTED_SOURCES.iter().copied());
    let dom_from_jar = cp.map_or(false, |cp| cp.holds_tasty_in(DOM_PACKAGE));
    let facades = SCALAJS_SOURCES
        .iter()
        .copied()
        .filter(|(path, _)| !(dom_from_jar && path.starts_with(DOM_STD_FILES)))
        .filter(|(path, _)| !opts.jvm || !TESTING_STD_FILES.contains(path));
    let input = |(path, text): (&'static str, &'static str), layer| {
        let index = stdindex::std_file_index(path).unwrap_or_else(|| panic!("{} is missing from the std index", path));
        frontend::StdInput { index, text, layer }
    };
    base.map(|f| input(f, typer::stdlib::Layer::Base)).chain(facades.map(|f| input(f, typer::stdlib::Layer::ScalaJs))).collect()
}

/// Reads the program, parses it and the std files it names, and lays every file out: the std
/// slots first, then the program's files, then the package blocks. Also the names the program
/// uses, for what the typer is told about the Scala.js layer.
pub(crate) fn parse_build(
    opts: &Options,
    cp: Option<&classpath::Classpath>,
    program: Vec<SourceFile>,
    interner: &mut intern::Interner,
    diags: &mut Diagnostics,
) -> (Vec<SourceFile>, Vec<ast::Ast>, Vec<Vec<usize>>, Vec<frontend::StdInput>, Vec<bool>, bool) {
    let std = std_layout(opts, cp);
    let n_std = std.len();
    // A std slot gets its text when it is parsed: with the program here, or when it enters.
    let mut files: Vec<SourceFile> = std.iter().map(|s| SourceFile { path: s.index.path.to_string(), text: String::new(), is_std: true, key: s.index.path.to_string() }).collect();
    files.extend(program);
    let parallel = frontend::parallel(&files);
    let mut slots: Vec<Option<Vec<ast::Ast>>> = (0..files.len()).map(|_| None).collect();
    let mut used = frontend::UsedNames::default();
    frontend::parse_files(&files, n_std..files.len(), &mut slots, interner, diags, Some(&mut used), parallel, usize::MAX, opts.syntax);
    let scalajs_unlocked = scalajs_jar(&opts.classpath) || frontend::names_scalajs(&used, interner);
    let selected = frontend::select_std(&std, &used, interner, scalajs_unlocked);
    for i in (0..n_std).filter(|&i| selected[i]) {
        files[i].text = std[i].text.to_string();
    }
    let todo = (0..n_std).filter(|&i| selected[i]);
    // The std files are few and small: a handful of workers parse them in the time it would
    // take to start a full set.
    frontend::parse_files(&files, todo, &mut slots, interner, diags, None, parallel, frontend::STD_WORKERS, parser::Syntax::default());
    let (asts, blocks_of) = frontend::lay_out(&mut files, slots, &std);
    let locked = used.defines_scalajs;
    (files, asts, blocks_of, std, selected, scalajs_unlocked && !locked)
}

/// Hands the typer the std slots of the build: the parsed ones enter with the program, the rest
/// when first named. A program that defines a Scala.js package itself keeps the facades out.
pub(crate) fn install_std(typer: &mut typer::Worker, std: Vec<frontend::StdInput>, selected: &[bool], blocks_of: &[Vec<usize>], scalajs_unlocked: bool) {
    let slots: Vec<typer::stdlib::StdSlot> = std
        .into_iter()
        .enumerate()
        .map(|(i, input)| typer::stdlib::StdSlot {
            index: input.index,
            text: input.text,
            file: source::FileId(i as u32),
            blocks: blocks_of[i].iter().map(|&b| source::FileId(b as u32)).collect(),
            layer: input.layer,
            parsed: std::sync::atomic::AtomicBool::new(selected[i]),
            entered: std::sync::atomic::AtomicBool::new(false), visible: std::sync::atomic::AtomicBool::new(false),
        })
        .collect();
    typer.set_std(slots, scalajs_unlocked);
}

const DOM_PACKAGE: &str = "org/scalajs/dom";

/// sbt's test interface, Scala.js's test bridge and the JUnit classes of Scala.js's test
/// runtime, ports of Scala 2 artifacts: the JVM's are sbt's and JUnit's Java ones, from their
/// jars, and its tests run in sbt's JVM.
pub(crate) const TESTING_STD_FILES: [&str; 3] = ["<std>/scalajs/junit.scala", "<std>/scalajs/test_bridge.scala", "<std>/scalajs/testing.scala"];
const DOM_STD_FILES: &str = "<std>/scalajs/dom";

/// Whether a Scala.js jar is on the classpath, which unlocks the facades layer.
fn scalajs_jar(classpath: &[String]) -> bool {
    classpath.iter().any(|p| p.contains("_sjs1_"))
}

/// The platform layer written for JavaScript alone: the JVM has the JDK's own classes.
const JS_ONLY_LAYER: [&str; 10] = [
    "<std>/concurrent.scala",
    "<std>/reflect_selectable.scala",
    "<std>/javalib/net.scala",
    "<std>/javalib/nio.scala",
    "<std>/javalib/nio_exceptions.scala",
    "<std>/javalib/stream.scala",
    "<std>/javalib/spliterator.scala",
    "<std>/javalib/optional.scala",
    "<std>/javalib/text.scala",
    "<std>/javalib/concurrent.scala",
];

/// Whether `name` of the package `pkg()` is a class, or an object of its statics, that only the
/// JavaScript layer defines, which on the JVM is the JDK's: a std file the JVM shares names the
/// JDK's class there (`Collection.stream`'s `java.util.stream.Stream` and `StreamSupport`).
pub(crate) fn js_only_class(name: &str, pkg: impl FnOnce() -> String) -> bool {
    static NAMES: std::sync::OnceLock<intern::FxMap<&'static str, Vec<&'static str>>> = std::sync::OnceLock::new();
    let names = NAMES.get_or_init(|| {
        let mut names: intern::FxMap<&'static str, Vec<&'static str>> = intern::FxMap::default();
        for f in JS_ONLY_LAYER.iter().filter_map(|path| stdindex::std_file_index(path)) {
            for &(p, n, _) in f.defines {
                names.entry(n).or_default().push(p);
            }
        }
        names
    });
    names.get(name).map_or(false, |pkgs| {
        let pkg = pkg();
        pkgs.iter().any(|&p| p == pkg)
    })
}

pub(crate) const STD_SOURCES: &[(&str, &str)] = &[
    ("<std>/core.scala", include_str!("../std/core.scala")),
    ("<std>/prelude.scala", include_str!("../std/prelude.scala")),
    ("<std>/sys.scala", include_str!("../std/sys.scala")),
    ("<std>/collections.scala", include_str!("../std/collections.scala")),
    ("<std>/iterable.scala", include_str!("../std/iterable.scala")),
    ("<std>/ordering.scala", include_str!("../std/ordering.scala")),
    ("<std>/javalib/string.scala", include_str!("../std/javalib/string.scala")),
    ("<std>/javalib/charset.scala", include_str!("../std/javalib/charset.scala")),
    ("<std>/strings.scala", include_str!("../std/strings.scala")),
    ("<std>/regex.scala", include_str!("../std/regex.scala")),
    ("<std>/vector.scala", include_str!("../std/vector.scala")),
    ("<std>/maps.scala", include_str!("../std/maps.scala")),
    ("<std>/immutable.scala", include_str!("../std/immutable.scala")),
    ("<std>/mutable.scala", include_str!("../std/mutable.scala")),
    ("<std>/math.scala", include_str!("../std/math.scala")),
    ("<std>/bignum.scala", include_str!("../std/bignum.scala")),
    ("<std>/exceptions.scala", include_str!("../std/exceptions.scala")),
    ("<std>/reflect.scala", include_str!("../std/reflect.scala")),
    ("<std>/enum.scala", include_str!("../std/enum.scala")),
    ("<std>/iarray.scala", include_str!("../std/iarray.scala")),
    ("<std>/classtag.scala", include_str!("../std/classtag.scala")),
    ("<std>/typetest.scala", include_str!("../std/typetest.scala")),
    ("<std>/nametransformer.scala", include_str!("../std/nametransformer.scala")),
    ("<std>/control.scala", include_str!("../std/control.scala")),
    ("<std>/try.scala", include_str!("../std/try.scala")),
    ("<std>/hashing.scala", include_str!("../std/hashing.scala")),
    ("<std>/abstract_collections.scala", include_str!("../std/abstract_collections.scala")),
    ("<std>/collections_more.scala", include_str!("../std/collections_more.scala")),
    ("<std>/runtime.scala", include_str!("../std/runtime.scala")),
    ("<std>/duration.scala", include_str!("../std/duration.scala")),
    ("<std>/concurrent.scala", include_str!("../std/concurrent.scala")),
    ("<std>/reflect_selectable.scala", include_str!("../std/reflect_selectable.scala")),
    ("<std>/console.scala", include_str!("../std/console.scala")),
    ("<std>/js.scala", include_str!("../std/js.scala")),
    ("<std>/annotation.scala", include_str!("../std/annotation.scala")),
    ("<std>/singleton.scala", include_str!("../std/singleton.scala")),
    ("<std>/value_of.scala", include_str!("../std/value_of.scala")),
    ("<std>/deriving.scala", include_str!("../std/deriving.scala")),
    ("<std>/mirrors.scala", include_str!("../std/mirrors.scala")),
    ("<std>/canequal.scala", include_str!("../std/canequal.scala")),
    ("<std>/partial.scala", include_str!("../std/partial.scala")),
    ("<std>/evidence.scala", include_str!("../std/evidence.scala")),
    ("<std>/notgiven.scala", include_str!("../std/notgiven.scala")),
    ("<std>/compiletime.scala", include_str!("../std/compiletime.scala")),
    ("<std>/compiletime_ops.scala", include_str!("../std/compiletime_ops.scala")),
    ("<std>/namedtuple.scala", include_str!("../std/namedtuple.scala")),
    ("<std>/javalib/lang.scala", include_str!("../std/javalib/lang.scala")),
    ("<std>/javalib/util.scala", include_str!("../std/javalib/util.scala")),
    ("<std>/javalib/util_more.scala", include_str!("../std/javalib/util_more.scala")),
    ("<std>/javalib/sorted.scala", include_str!("../std/javalib/sorted.scala")),
    ("<std>/javalib/atomic.scala", include_str!("../std/javalib/atomic.scala")),
    ("<std>/javalib/nio.scala", include_str!("../std/javalib/nio.scala")),
    ("<std>/javalib/nio_exceptions.scala", include_str!("../std/javalib/nio_exceptions.scala")),
    ("<std>/javalib/stream.scala", include_str!("../std/javalib/stream.scala")),
    ("<std>/javalib/spliterator.scala", include_str!("../std/javalib/spliterator.scala")),
    ("<std>/javalib/optional.scala", include_str!("../std/javalib/optional.scala")),
    ("<std>/javalib/buffers.scala", include_str!("../std/javalib/buffers.scala")),
    ("<std>/javalib/charbuffer.scala", include_str!("../std/javalib/charbuffer.scala")),
    ("<std>/javalib/math.scala", include_str!("../std/javalib/math.scala")),
    ("<std>/javalib/text.scala", include_str!("../std/javalib/text.scala")),
    ("<std>/javalib/net.scala", include_str!("../std/javalib/net.scala")),
    ("<std>/javalib/concurrent.scala", include_str!("../std/javalib/concurrent.scala")),
    ("<std>/jdk.scala", include_str!("../std/jdk.scala")),
    ("<std>/buffer.scala", include_str!("../std/buffer.scala")),
];

/// The Scala.js facade layer, `scala.scalajs.js` and `org.scalajs.dom` in teq's facade syntax:
/// unlocked when a program names either package or a Scala.js jar is on the classpath, and its
/// files entered by name from then on, so that hello world and the benchmark pay nothing for it.
pub(crate) const SCALAJS_SOURCES: &[(&str, &str)] = &[
    ("<std>/scalajs/annotation.scala", include_str!("../std/scalajs/annotation.scala")),
    ("<std>/scalajs/concurrent.scala", include_str!("../std/scalajs/concurrent.scala")),
    ("<std>/scalajs/converters.scala", include_str!("../std/scalajs/converters.scala")),
    ("<std>/scalajs/dom.scala", include_str!("../std/scalajs/dom.scala")),
    ("<std>/scalajs/dom_compat.scala", include_str!("../std/scalajs/dom_compat.scala")),
    ("<std>/scalajs/dom_elements.scala", include_str!("../std/scalajs/dom_elements.scala")),
    ("<std>/scalajs/dom_events.scala", include_str!("../std/scalajs/dom_events.scala")),
    ("<std>/scalajs/dom_fetch.scala", include_str!("../std/scalajs/dom_fetch.scala")),
    ("<std>/scalajs/dom_files.scala", include_str!("../std/scalajs/dom_files.scala")),
    ("<std>/scalajs/dom_lists.scala", include_str!("../std/scalajs/dom_lists.scala")),
    ("<std>/scalajs/dom_media.scala", include_str!("../std/scalajs/dom_media.scala")),
    ("<std>/scalajs/dom_nodes.scala", include_str!("../std/scalajs/dom_nodes.scala")),
    ("<std>/scalajs/dom_observers.scala", include_str!("../std/scalajs/dom_observers.scala")),
    ("<std>/scalajs/dom_svg.scala", include_str!("../std/scalajs/dom_svg.scala")),
    ("<std>/scalajs/dom_window.scala", include_str!("../std/scalajs/dom_window.scala")),
    ("<std>/scalajs/js.scala", include_str!("../std/scalajs/js.scala")),
    ("<std>/scalajs/linking_info.scala", include_str!("../std/scalajs/linking_info.scala")),
    ("<std>/scalajs/ops.scala", include_str!("../std/scalajs/ops.scala")),
    ("<std>/scalajs/reflect.scala", include_str!("../std/scalajs/reflect.scala")),
    ("<std>/scalajs/reflect_annotation.scala", include_str!("../std/scalajs/reflect_annotation.scala")),
    ("<std>/scalajs/special.scala", include_str!("../std/scalajs/special.scala")),
    ("<std>/scalajs/junit.scala", include_str!("../std/scalajs/junit.scala")),
    ("<std>/scalajs/test_bridge.scala", include_str!("../std/scalajs/test_bridge.scala")),
    ("<std>/scalajs/testing.scala", include_str!("../std/scalajs/testing.scala")),
    ("<std>/scalajs/thenable.scala", include_str!("../std/scalajs/thenable.scala")),
    ("<std>/scalajs/timers.scala", include_str!("../std/scalajs/timers.scala")),
    ("<std>/scalajs/typedarray.scala", include_str!("../std/scalajs/typedarray.scala")),
    ("<std>/scalajs/undefor.scala", include_str!("../std/scalajs/undefor.scala")),
];

/// `scala.quoted`: entered when a quote, a splice, an import or a macro's signature names it.
pub(crate) const QUOTED_SOURCES: &[(&str, &str)] = &[
    ("<std>/quoted/quotes.scala", include_str!("../std/quoted/quotes.scala")),
    ("<std>/quoted/reflect.scala", include_str!("../std/quoted/reflect.scala")),
];

/// What generated JVM code calls and the JS runtime provides on the other side, linked against
/// scala-library's collections.
pub(crate) const JVM_STD_SOURCES: &[(&str, &str)] = &[
    ("<std>/jvm.scala", include_str!("../std/jvm.scala")),
    ("<std>/jvm_scala_library.scala", include_str!("../std/jvm_scala_library.scala")),
    ("<std>/jvm_buffer.scala", include_str!("../std/jvm_buffer.scala")),
];

/// The builtin layer, which stays under `--std=scala-library` and which the jar's classes meet
/// rather than replace: the members of the builtin classes, `java.lang.String`'s methods as
/// intrinsics, the `Throwable` hierarchy of `java.lang` (which `throw` and `catch` need and no
/// TASTy file defines), `Class` and `ClassTag`, and the Java platform layer for JavaScript.
pub(crate) const SCALA_LIBRARY_LAYER: &[(&str, &str)] = &[
    ("<std>/core.scala", include_str!("../std/core.scala")),
    ("<std>/javalib/string.scala", include_str!("../std/javalib/string.scala")),
    ("<std>/javalib/charset.scala", include_str!("../std/javalib/charset.scala")),
    ("<std>/exceptions.scala", include_str!("../std/exceptions.scala")),
    ("<std>/reflect.scala", include_str!("../std/reflect.scala")),
    ("<std>/enum.scala", include_str!("../std/enum.scala")),
    ("<std>/iarray.scala", include_str!("../std/iarray.scala")),
    ("<std>/classtag.scala", include_str!("../std/classtag.scala")),
    ("<std>/singleton.scala", include_str!("../std/singleton.scala")),
    ("<std>/value_of.scala", include_str!("../std/value_of.scala")),
    ("<std>/js.scala", include_str!("../std/js.scala")),
    ("<std>/javalib/lang.scala", include_str!("../std/javalib/lang.scala")),
    ("<std>/javalib/util.scala", include_str!("../std/javalib/util.scala")),
    ("<std>/javalib/util_more.scala", include_str!("../std/javalib/util_more.scala")),
    ("<std>/javalib/sorted.scala", include_str!("../std/javalib/sorted.scala")),
    ("<std>/javalib/atomic.scala", include_str!("../std/javalib/atomic.scala")),
    ("<std>/javalib/stream.scala", include_str!("../std/javalib/stream.scala")),
    ("<std>/javalib/spliterator.scala", include_str!("../std/javalib/spliterator.scala")),
    ("<std>/javalib/optional.scala", include_str!("../std/javalib/optional.scala")),
    ("<std>/javalib/buffers.scala", include_str!("../std/javalib/buffers.scala")),
    ("<std>/javalib/charbuffer.scala", include_str!("../std/javalib/charbuffer.scala")),
    ("<std>/javalib/math.scala", include_str!("../std/javalib/math.scala")),
    ("<std>/javalib/text.scala", include_str!("../std/javalib/text.scala")),
    ("<std>/javalib/net.scala", include_str!("../std/javalib/net.scala")),
    ("<std>/javalib/concurrent.scala", include_str!("../std/javalib/concurrent.scala")),
    ("<std>/library/mirrors.scala", include_str!("../std/library/mirrors.scala")),
    ("<std>/library/product.scala", include_str!("../std/library/product.scala")),
    ("<std>/buffer.scala", include_str!("../std/buffer.scala")),
];

/// Which standard library a build compiles against: teq's own, written for JavaScript, or
/// scala-library itself, read from its jar and compiled from its TASTy bodies. A JVM build has
/// the second alone, linked against the jar's bytecode (`parse_options_from`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StdMode {
    Lean,
    ScalaLibrary,
}

impl StdMode {
    pub(crate) fn sources(self) -> &'static [(&'static str, &'static str)] {
        match self {
            StdMode::Lean => STD_SOURCES,
            StdMode::ScalaLibrary => SCALA_LIBRARY_LAYER,
        }
    }

    pub(crate) fn parse(text: &str) -> Option<StdMode> {
        match text {
            "lean" => Some(StdMode::Lean),
            "scala-library" => Some(StdMode::ScalaLibrary),
            _ => None,
        }
    }
}

pub(crate) struct Options {
    pub command: String,
    pub inputs: Vec<String>,
    pub excludes: Vec<String>,
    pub output: Option<String>,
    pub split: Option<String>,
    /// `watch --check`: the session types and reports every diagnostic, writing nothing.
    pub check: bool,
    /// `watch --check --index`: the session keeps the navigation index and answers the
    /// language server's queries, its positions counted as `positions` says.
    pub index: bool,
    /// `watch --wait-for-build`: the first build waits for the first `build` command, the texts
    /// before it taken in (the language server's sessions).
    pub wait_for_build: bool,
    pub positions: watch::Positions,
    pub module_per_file: Vec<String>,
    pub hot: bool,
    pub main: Option<String>,
    /// `--cacheable-state a.B` and the `cacheable-state` of the `teq.toml`: the objects a watch
    /// session keeps across its builds, declared by the macros' author to hold nothing but a cache.
    pub cacheable_state: Vec<String>,
    /// `--no-known-caches`: the libraries' known caches (`typer::KNOWN_CACHES`) count as none.
    pub no_known_caches: bool,
    /// `--macro-state per-worker`: each worker's macro runs keep the state they change to the
    /// worker, where by default a change of state other runs share gives the build to one worker.
    pub macro_state_per_worker: bool,
    pub program_args: Vec<String>,
    pub timings: bool,
    pub werror: bool,
    /// `--wunused <kinds>`, scalac's `-Wunused`: `imports` (or `all`) reports the unused imports
    /// as warnings (`typer::unused`); the other kinds are accepted and do nothing yet.
    pub wunused_imports: bool,
    pub dump_tokens: bool,
    /// `--dump-defs`: each file's definitions and the parser's recovery (`parser::dump`), in the
    /// place of the build.
    pub dump_defs: bool,
    /// The dialect flags of `--dialect`, `--strict-equality` and the `teq.toml` next to the sources.
    pub dialect: dialect::Dialect,
    /// `--max-inlines` or the `max-inlines` of the `teq.toml`: scalac's `-Xmax-inlines`.
    pub max_inlines: u32,
    pub jvm: bool,
    /// `teq interp`: the program runs under the interpreter, without an output file.
    pub interp: bool,
    pub classpath: Vec<String>,
    pub std: StdMode,
    pub release: bool,
    /// The JDK release read from `ct.sym`, its newest by default.
    pub jdk_release: Option<u32>,
    /// `--java-output-version`: the Java release the class files of a JVM build are written for.
    pub java_output_version: u32,
    pub size_report: bool,
    /// `--no-outline`: every inline expansion and anonymous class written at its site.
    pub no_outline: bool,
    pub syntax: parser::Syntax,
    /// `--profile`: where the type phase's time went, and the file its whole table goes to.
    pub profile: bool,
    pub profile_json: Option<String>,
    /// `--no-cache` or `TEQ_NO_CACHE`: the jars are read without their cache.
    pub no_cache: bool,
    /// `--own root`: the inputs whose class files a JVM build writes, or whose classes a check
    /// session answers; the other inputs are typed with them and left to the build that owns
    /// them.
    pub own: Vec<String>,
    /// `--all-mains`: a JVM build needs no entry point and writes the class the `java` launcher
    /// runs for every entry point, as scalac does, in the place of `TeqMain` for one.
    pub all_mains: bool,
    /// `--threads N`: the workers of the typer's body phase; 0 for the default, which is one.
    pub threads: usize,
    /// `--products <dir>`: the module-product mode (`products.rs`).
    pub products: Option<String>,
    /// `--removed <source>`: a source of the module that is gone, whose products a build into
    /// the module's own directory drops (docs/TARGETS.md, "The contract between the plugin and
    /// the compiler").
    pub removed: Vec<String>,
    /// `--analysis-version 2`: the answer carries the API graph of the files it covers
    /// (`jvm::api`), a one-shot build's on stdout; `3`: and the dependencies of their classes
    /// (`typer::deps`).
    pub analysis_version: Option<u32>,
    /// `--sourceroot <dir>`: what the pickles' source paths are relative to, scalac's
    /// `-sourceroot`; the working directory by default.
    pub sourceroot: Option<String>,
}

/// Whether scalac's `-Wunused:<kinds>` choices report unused imports, read in order as scalac
/// reads them (`-imports` takes it back); None for a kind scalac does not know.
fn wunused_imports(kinds: &str) -> Option<bool> {
    const KNOWN: [&str; 12] = ["nowarn", "all", "imports", "privates", "locals", "explicits", "implicits", "params", "linted", "strict-no-implicit-warn", "unsafe-warn-patvars", "patvars"];
    let mut on = false;
    for kind in kinds.split(',').map(str::trim).filter(|k| !k.is_empty()) {
        let (negated, name) = match kind.strip_prefix('-') {
            Some(n) => (true, n),
            None => (false, kind),
        };
        if !KNOWN.contains(&name) {
            return None;
        }
        match name {
            "imports" | "all" | "linted" => on = !negated,
            _ => {}
        }
    }
    Some(on)
}

/// The usage of `teq compiler`: on stderr with exit 2 for a bad command line, on stdout with
/// exit 0 for `--help`.
const COMPILER_USAGE: &str = "usage: teq compiler <build|check|watch> <files or directories...> [--target js|jvm] [-o out.js | --split dir [--module-per-file pkg,...] [--hot] | --check] [--release] [--no-outline] [--exclude path]... [--own root]... [--all-mains] [--products dir] [--removed source]... [--sourceroot dir] [--classpath a.jar:b.jar] [--no-cache] [--std lean|scala-library] [--java-output-version N] [--main name] [--cacheable-state a.B]... [--no-known-caches] [--macro-state ordered|per-worker] [--dialect flags] [--strict-equality] [--kind-projector] [--werror] [--threads n] [--time] [--profile[=file.json]] [--size-report]\n\
         \n\
         @<file>  stands for the file's lines, one argument per line, anywhere before a `--` (what\n\
         \x20        follows one, on the line or from a file, is the program's as written; UTF-8,\n\
         \x20        `\\r\\n` or `\\n`, each line as it is: no quoting, no comments, no nesting);\n\
         \x20        a source or other path whose name starts with @ is given with a directory in\n\
         \x20        front, ./@Name.scala, and an argument file so named as @./@name\n\
         \n\
         build   compile to a single JavaScript file (default out/main.js, or out/main.mjs for\n\
         \x20       an ES module, which is what a program with @jsImport or @jsExport becomes),\n\
         \x20       which node runs; `teq interp` runs a program without a build\n\
         check   type check only\n\
         watch   build with --split, then rebuild on `build` lines read from stdin (paths of the\n\
         \x20       changed files on the following lines, ended by an empty line, or none to\n\
         \x20       check every file) and answer each with a JSON line; `quit` ends it; with\n\
         \x20       --check the session only types, JVM programs included, and every answer\n\
         \x20       lists the diagnostics, and `text <path> <bytes>` followed by the bytes\n\
         \x20       stands in for the file on disk until `text <path> 0` (docs/TARGETS.md);\n\
         \x20       `teq lsp` runs one `watch --check --index` per project of the workspace\n\
         \n\
         --target  jvm writes class files in the place of JavaScript: a jar when -o ends in .jar\n\
         \x20         (default out/main.jar), a directory of .class files otherwise, linked against\n\
         \x20         scala-library's bytecode, so the jar runs beside it (java -Xss512m -cp\n\
         \x20         out/main.jar:scala-library.jar TeqMain; the stack is for the std's recursion);\n\
         \x20         `watch` with --target jvm and -o keeps a JVM build resident, rewriting the\n\
         \x20         class files that changed and answering with the classes of each file\n\
         \x20         (docs/TARGETS.md)\n\
         --own     a JVM build writes the class files of the inputs under this root only (repeatable),\n\
         \x20         a check session answers their classes; the other inputs are typed with them\n\
         --products  a module's products in a directory, for the modules built against it (a\n\
         \x20         JVM build's class files, or with `check` a Scala.js module's pickles alone):\n\
         \x20         every definition kept, a .tasty beside each top-level class and\n\
         \x20         teq-products.json listing them (docs/TARGETS.md); a directory of products on\n\
         \x20         --classpath is another module, called and never compiled again; the directory\n\
         \x20         itself on --classpath makes the build part of the module's sources against its\n\
         \x20         other products, published together with the manifest after the build\n\
         --removed  a source of the module that is gone: its products leave the directory; with\n\
         \x20         no sources, the manifest alone is rewritten (repeatable)\n\
         --sourceroot  what the source paths in the pickles are relative to (scalac's\n\
         \x20         -sourceroot); the working directory by default\n\
         --analysis-version 2  a JVM build or a check answers the API graph of its classes for\n\
         \x20         zinc on stdout, a session in every answer (docs/TARGETS.md, \"The analysis graph\");\n\
         \x20         3 answers the dependencies of its classes too\n\
         --all-mains  a JVM build needs no entry point and writes, as scalac does, the class the\n\
         \x20         java launcher runs for every entry point in the place of TeqMain for one\n\
         --split   writes a directory of ES modules instead: main.mjs (the entry), rt.mjs (the\n\
         \x20         runtime), std.mjs and one module per package; only the modules that changed\n\
         \x20         since the last build are rewritten\n\
         --module-per-file   packages named or below one of the comma-separated prefixes get one module per\n\
         \x20         source file (a.b.File.mjs) in the place of one per package\n\
         --hot     modules run $init and $enums once per instance and export $hot(), which calls\n\
         \x20         the accessors of their objects, for a development server that re-executes\n\
         \x20         edited modules in a page that has booted\n\
         --release JavaScript for production: the members of Scala classes get short names, the\n\
         \x20         output has no indentation or line breaks and the runtime no comments\n\
         --no-outline  writes every inline expansion and every anonymous class it makes at its\n\
         \x20         site, as the JVM target does, in the place of one function per shape and one\n\
         \x20         class per body (docs/TARGETS.md, \"The JavaScript output\"); for debugging\n\
         --exclude leaves out the files whose path starts with or ends with the given text\n\
         --classpath  jars and class directories whose TASTy and class files supply the classes\n\
         \x20         the program names; a check reads what the program touches and nothing else;\n\
         \x20         the JDK is read from its lib/ct.sym on the first java.* lookup, at its newest\n\
         \x20         release unless --release <n> says\n\
         --no-cache  reads the jars without the jar cache, the inflated TASTy files of each jar\n\
         \x20         that earlier builds kept under $TEQ_CACHE_DIR (default ~/Library/Caches/teq\n\
         \x20         on macOS, ~/.cache/teq elsewhere); TEQ_NO_CACHE=1 does the same\n\
         --std     which standard library a JavaScript program compiles against: lean (the\n\
         \x20         default), teq's own, or scala-library, read from its jar and compiled from its\n\
         \x20         TASTy bodies; the builtins and the platform layer stay either way. A JVM\n\
         \x20         build always links against scala-library (--std=lean is refused there). The\n\
         \x20         jar is named with --classpath or taken from the coursier cache (the newest 3.x)\n\
         --java-output-version  scalac's -java-output-version: the Java release whose class file\n\
         \x20         version a JVM build writes, 17 (the default) to 24\n\
         --main    runs the @main method or the object of that name when the program has several\n\
         --cacheable-state  an object of the program, of a jar or of the std, by its qualified name,\n\
         \x20         that a macro's author declares to hold nothing but a cache: a watch session\n\
         \x20         keeps it and what it reaches across its retypes where every other object\n\
         \x20         of the program is made anew (repeatable; `cacheable-state = [...]` in the\n\
         \x20         teq.toml; docs/TARGETS.md, \"Watch mode\")\n\
         --no-known-caches  the libraries' objects teq knows to hold nothing but a cache count as\n\
         \x20         undeclared for the watch over the state macro runs share with several workers\n\
         --macro-state  per-worker: with several workers, a macro's run that changes state other\n\
         \x20         runs share (a counter, a registry) keeps the change to its worker, whose later\n\
         \x20         runs alone see it, a departure from scalac's order; ordered, the default: the\n\
         \x20         build is typed again by one worker, in scalac's order\n\
         --dialect  flags that turn costly constructs off, comma-separated, or `strict` for all of\n\
         \x20         them: no-implicit-conversions, no-overloading, no-inline, explicit-result-types,\n\
         \x20         no-scala2-implicits, strict-equality, no-nonlocal-returns; a teq.toml next to\n\
         \x20         the sources sets them for a project (see the README)\n\
         --strict-equality  scalac's -language:strictEquality: every == needs a CanEqual instance\n\
         --kind-projector  scalac's -Xkind-projector: a `*` among the arguments of a type makes it a\n\
         \x20         type lambda (`Either[String, *]` for `[X] =>> Either[String, X]`)\n\
         --max-inlines  scalac's -Xmax-inlines: how deep inline calls may nest (32 by default);\n\
         \x20         a `max-inlines = n` in the teq.toml sets it for a project\n\
         --werror  fails the build on warnings\n\
         --wunused scalac's -Wunused with its comma-separated kinds: `imports` or `all` warns on\n\
         \x20         each import selector that no name resolves through (`unused import`); the\n\
         \x20         other kinds are accepted and ignored for now\n\
         --threads how many workers type the bodies (TEQ_THREADS too): by default one for a\n\
         \x20         program under 512 KiB of source or one whose largest top-level definition\n\
         \x20         holds more than half of it, else the least of the cores (at most\n\
         \x20         TEQ_WORKERS), 8 and one per GiB of memory above the first; 1 types every\n\
         \x20         build on one worker\n\
         --time    reports where the time went\n\
         --profile reports where the time of the type phase went: given searches, overload\n\
         \x20         resolutions, conversion searches, inline expansions and the rest, per kind\n\
         \x20         and per site, each with the hint that would remove the cost;\n\
         \x20         --profile=<file.json> also writes the whole table\n\
         --size-report  reports where the bytes of the JavaScript output went";

fn usage() -> ! {
    eprintln!("{}", COMPILER_USAGE);
    std::process::exit(2);
}

fn compiler_help() -> ! {
    println!("{}", COMPILER_USAGE);
    std::process::exit(0);
}

/// The usage of `teq interp`, with the options it takes; the others are refused by name
/// (`interp_refuses`).
const INTERP_USAGE: &str = "usage: teq interp <files or directories...> [options] [-- args]\n\
         \n\
         Runs the program in the compiler's own interpreter: the files are typed as for a build, the\n\
         entry point runs, what follows -- reaches it as its arguments, and the exit code is the\n\
         program's (the status it gives sys.exit, 1 for an exception it did not catch or an error met\n\
         on the way, 3 for what the interpreter does not execute). The compiler writes nothing; the\n\
         program has its process's working directory, environment and files, as on the JVM, through\n\
         java.nio.file. No node or JVM is needed.\n\
         \n\
         --main name   runs the @main method or the object of that name when the program has several\n\
         --exclude path  leaves out the files whose path starts with or ends with the text (repeatable)\n\
         --classpath  jars and class directories whose TASTy and class files supply the classes the\n\
         \x20         program names; the JDK is read from its lib/ct.sym at its newest release unless\n\
         \x20         --release <n> says\n\
         --release [n]  with a number, the JDK release read from ct.sym; alone, the production mode a\n\
         \x20         program observes through LinkingInfo.productionMode\n\
         --no-cache   reads the jars without the jar cache (TEQ_NO_CACHE=1 does the same)\n\
         --sourceroot dir  what the pickles' source paths are relative to (scalac's -sourceroot)\n\
         --dialect flags, --strict-equality, --kind-projector, --max-inlines n\n\
         \x20         the typer's settings, as teq compiler takes them (teq compiler --help)\n\
         --cacheable-state a.B, --no-known-caches, --macro-state ordered|per-worker\n\
         \x20         the rules for the state macro runs share with several workers, as teq compiler\n\
         \x20         takes them\n\
         --threads n  how many workers type the bodies (TEQ_THREADS too); one by default for a\n\
         \x20         small program\n\
         --werror     fails on warnings; --wunused <kinds> warns on unused imports (scalac's -Wunused)\n\
         --time       reports where the time went, the run phase included\n\
         --profile[=file.json]  reports where the time of the type phase went, after the run, which\n\
         \x20         types the std's bodies the program reaches\n\
         --dump-tokens, --dump-defs  the parser's tokens or definitions, in the place of the run\n\
         --no-outline  accepted and does nothing: the interpreter writes no outlines\n\
         \n\
         The options of a build (-o, --split, --target, --std, --products, --own, --check and the\n\
         rest of teq compiler's) have no meaning here and are refused";

fn interp_usage() -> ! {
    eprintln!("{}", INTERP_USAGE);
    std::process::exit(2);
}

fn interp_help() -> ! {
    println!("{}", INTERP_USAGE);
    std::process::exit(0);
}

/// `TEQ_PANIC_REPORT=plain`: a panic's report without the thread, whose number and name differ
/// from run to run (which worker meets a failure of the parallel path is the schedule's, the first
/// on the calling thread and the others on threads of their own), so that a runner comparing two
/// runs' failures (`tests/fork-one.sh`) compares their output byte for byte.
fn plain_panic_reports() {
    std::panic::set_hook(Box::new(|info| {
        let at = info.location().map_or(String::new(), |l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
        let msg = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
        eprintln!("a thread panicked at {}:\n{}", at, msg);
        let trace = std::backtrace::Backtrace::capture();
        if trace.status() == std::backtrace::BacktraceStatus::Captured {
            eprintln!("{}", trace);
        }
    }));
}

fn print_version() -> ! {
    // The assertion-enabled build (the `checks` profile) says so.
    let checks = if cfg!(debug_assertions) { " assertions" } else { "" };
    match env!("TEQ_GIT_HASH") {
        "" => println!("teq {}{}", env!("CARGO_PKG_VERSION"), checks),
        hash => println!("teq {} {}{}", env!("CARGO_PKG_VERSION"), hash, checks),
    }
    std::process::exit(0);
}

fn parse_options() -> Options {
    parse_options_from(argfile::args().iter().cloned())
}

/// The binary's top level, by the first word: `--version` and `--help`; `tasty`, `classfile`
/// and `lsp`; `compiler <build|check|watch>` and `interp`, whose options this parses; else a
/// project verb, or the prefix options that come before one, which `task::run` serves with the
/// export found above the working directory. No word, or an unknown one, gets the usage.
fn parse_options_from(args: impl Iterator<Item = String>) -> Options {
    let args: Vec<String> = args.collect();
    let Some(first) = args.first() else { task::run(&[]) };
    match first.as_str() {
        "--version" => print_version(),
        "--help" | "-h" => task::help(),
        "tasty" => tasty::cli::run(&args[1..]),
        "classfile" => classfile::cli::run(&args[1..]),
        "lsp" => lsp::run(&args[1..]),
        "compiler" => parse_compiler_options(&args[1..]),
        "interp" => parse_file_options("interp", &args[1..]),
        _ if task::takes(first) => task::run(&args),
        _ => task::unknown(first),
    }
}

/// `teq compiler <build|check|watch> ...`: the compiler's usage without a verb, on stdout
/// with `--help`, else the verb's options.
fn parse_compiler_options(args: &[String]) -> Options {
    match args.first().map(String::as_str) {
        Some("build" | "check" | "watch") => parse_file_options(&args[0], &args[1..]),
        Some("--help" | "-h") => compiler_help(),
        _ => usage(),
    }
}

/// The usage of the file command `command`, exit 2.
fn usage_for(command: &str) -> ! {
    if command == "interp" {
        interp_usage()
    } else {
        usage()
    }
}

/// Why `teq interp` refuses the option `a`, or None for one it takes.
fn interp_refuses(a: &str) -> Option<&'static str> {
    let name = a.split_once('=').map_or(a, |(name, _)| name);
    match name {
        "-o" | "--split" | "--module-per-file" | "--hot" | "--size-report" | "--products" | "--own" | "--all-mains" | "--removed" | "--analysis-version" | "--java-output-version" => Some("is a build's: the interpreter runs the program and writes nothing"),
        "--target" => Some("belongs to teq compiler: the interpreter is a target of its own"),
        "--std" | "--no-std" => Some("belongs to teq compiler: the interpreter runs on the lean std"),
        "--check" | "--index" | "--wait-for-build" | "--positions" => Some("belongs to teq compiler watch: the interpreter runs once and keeps no session"),
        _ => None,
    }
}

/// The options of `teq compiler build|check|watch` and of `teq interp`, whose typing mode is
/// the interpreter's (`Options::interp`) and whose refused options get one line each.
fn parse_file_options(command: &str, args: &[String]) -> Options {
    let interp = command == "interp";
    if interp && matches!(args.first().map(String::as_str), Some("--help" | "-h")) {
        interp_help();
    }
    let mut o = Options {
        command: command.to_string(),
        inputs: Vec::new(),
        excludes: Vec::new(),
        output: None,
        split: None,
        check: false,
        index: false,
        wait_for_build: false,
        positions: watch::Positions::Utf16,
        module_per_file: Vec::new(),
        hot: false,
        main: None,
        cacheable_state: Vec::new(),
        no_known_caches: false,
        macro_state_per_worker: false,
        program_args: Vec::new(),
        timings: false,
        werror: false,
        wunused_imports: false,
        dump_tokens: false,
        dump_defs: false,
        dialect: dialect::Dialect::default(),
        max_inlines: typer::MAX_INLINES,
        jvm: false,
        interp,
        classpath: Vec::new(),
        std: StdMode::Lean,
        release: false,
        jdk_release: None,
        java_output_version: jvm::classfile::DEFAULT_OUTPUT_VERSION,
        size_report: false,
        no_outline: false,
        syntax: parser::Syntax::default(),
        profile: false,
        profile_json: None,
        no_cache: jarcache::disabled_by_env(),
        own: Vec::new(),
        all_mains: false,
        threads: 0,
        products: None,
        removed: Vec::new(),
        sourceroot: None,
        analysis_version: None,
    };
    let mut flags: Vec<String> = Vec::new();
    let mut max_inlines = None;
    let mut std_given = None;
    let mut args = args.iter().cloned().peekable();
    while let Some(a) = args.next() {
        if interp {
            if let Some(why) = interp_refuses(&a) {
                eprintln!("teq interp: {} {}", a, why);
                std::process::exit(2);
            }
        }
        match a.as_str() {
            "-o" => o.output = args.next(),
            "--split" => o.split = args.next(),
            "--check" => o.check = true,
            "--index" => o.index = true,
            "--wait-for-build" => o.wait_for_build = true,
            "--positions" => match args.next().as_deref() {
                Some("utf-16") => o.positions = watch::Positions::Utf16,
                Some("utf-8") => o.positions = watch::Positions::Utf8,
                _ => {
                    eprintln!("--positions takes utf-16 or utf-8");
                    std::process::exit(2);
                }
            },
            "--module-per-file" => match args.next() {
                Some(list) => o.module_per_file.extend(list.split(',').filter(|p| !p.is_empty()).map(str::to_string)),
                None => usage_for(command),
            },
            "--hot" => o.hot = true,
            "--target" => match args.next().as_deref() {
                Some("jvm") => o.jvm = true,
                Some("js") => o.jvm = false,
                Some("interp") => {
                    eprintln!("--target interp is `teq interp <files or directories...> [options] [-- args]`: the interpreter is a verb of its own");
                    std::process::exit(2);
                }
                _ => usage_for(command),
            },
            "--exclude" => o.excludes.extend(args.next()),
            "--own" => o.own.extend(args.next().map(|r| r.trim_end_matches('/').to_string())),
            "--products" => match args.next() {
                Some(d) => o.products = Some(d),
                None => usage_for(command),
            },
            "--removed" => match args.next() {
                Some(s) => o.removed.push(s),
                None => usage_for(command),
            },
            "--sourceroot" => match args.next() {
                Some(d) => o.sourceroot = Some(d),
                None => usage_for(command),
            },
            "--all-mains" => o.all_mains = true,
            "--classpath" => match args.next() {
                Some(list) => o.classpath.extend(list.split(classpath::SEPARATOR).filter(|p| !p.is_empty()).map(str::to_string)),
                None => usage_for(command),
            },
            "--std" => match args.next().and_then(|m| StdMode::parse(&m)) {
                Some(mode) => std_given = Some(mode),
                None => usage_for(command),
            },
            _ if a.starts_with("--std=") => match StdMode::parse(&a["--std=".len()..]) {
                Some(mode) => std_given = Some(mode),
                None => usage_for(command),
            },
            "--no-std" => {
                eprintln!("--no-std was replaced by --std=scala-library");
                std::process::exit(2);
            }
            "--main" => o.main = args.next(),
            "--strict-equality" => flags.push(dialect::STRICT_EQUALITY.to_string()),
            "--max-inlines" => match args.next().and_then(|n| n.parse::<u32>().ok()) {
                Some(n) if n > 0 => max_inlines = Some(n),
                _ => usage_for(command),
            },
            "--dialect" => match args.next() {
                Some(list) => flags.push(list),
                None => usage_for(command),
            },
            "--" => {
                if !interp {
                    eprintln!("-- has no meaning under teq compiler: a program's arguments go to node or java after the build, or to `teq interp <files> -- args`");
                    std::process::exit(2);
                }
                o.program_args.extend(args.by_ref());
                break;
            }
            "--time" => o.timings = true,
            "--cacheable-state" => match args.next() {
                Some(name) => o.cacheable_state.push(name),
                None => usage_for(command),
            },
            "--no-known-caches" => o.no_known_caches = true,
            "--macro-state" => match args.next().as_deref() {
                Some("per-worker") => o.macro_state_per_worker = true,
                Some("ordered") => o.macro_state_per_worker = false,
                _ => usage_for(command),
            },
            "--release" => match args.peek().and_then(|n| n.parse::<u32>().ok()) {
                Some(n) => {
                    o.jdk_release = Some(n);
                    args.next();
                }
                None => o.release = true,
            },
            "--java-output-version" => {
                let given = args.next().unwrap_or_default();
                match given.parse::<u32>().ok().filter(|n| jvm::classfile::OUTPUT_VERSIONS.contains(n)) {
                    Some(n) => o.java_output_version = n,
                    None => {
                        let versions = jvm::classfile::OUTPUT_VERSIONS;
                        eprintln!("{} is not a valid choice for --java-output-version ({} to {})", given, versions.start(), versions.end());
                        std::process::exit(2);
                    }
                }
            }
            "--size-report" => o.size_report = true,
            "--no-outline" => o.no_outline = true,
            "--kind-projector" => o.syntax.kind_projector = true,
            "--profile" => o.profile = true,
            _ if a.starts_with("--profile=") => {
                o.profile = true;
                o.profile_json = Some(a["--profile=".len()..].to_string());
            }
            "--werror" => o.werror = true,
            "--wunused" => match args.next().and_then(|kinds| wunused_imports(&kinds)) {
                Some(on) => o.wunused_imports = on,
                None => usage_for(command),
            },
            "--threads" => match args.next().and_then(|n| n.parse::<usize>().ok()) {
                Some(n) if n >= 1 => o.threads = n,
                _ => usage_for(command),
            },
            "--no-cache" => o.no_cache = true,
            "--dump-tokens" => o.dump_tokens = true,
            "--dump-defs" => o.dump_defs = true,
            "--analysis-version" => match args.next().and_then(|n| n.parse::<u32>().ok()) {
                Some(n) => o.analysis_version = Some(n),
                None => usage_for(command),
            },
            _ if a.starts_with("--analysis-version=") => match a["--analysis-version=".len()..].parse::<u32>() {
                Ok(n) => o.analysis_version = Some(n),
                Err(_) => usage_for(command),
            },
            _ if a.starts_with('-') => usage_for(command),
            _ => o.inputs.push(a),
        }
    }
    if (o.inputs.is_empty() && o.products.is_none()) || (o.output.is_some() && o.split.is_some()) || (o.split.is_none() && (o.hot || !o.module_per_file.is_empty())) || (o.jvm && o.split.is_some()) {
        usage_for(command);
    }
    if o.index && !o.check {
        eprintln!("--index belongs to `teq compiler watch --check`");
        std::process::exit(2);
    }
    if o.wait_for_build && o.command != "watch" {
        eprintln!("--wait-for-build belongs to `teq compiler watch`");
        std::process::exit(2);
    }
    o.syntax.index = o.index;
    if o.check && (o.command != "watch" || o.split.is_some() || o.output.is_some()) {
        eprintln!("--check belongs to `teq compiler watch` and takes no --split or -o: the session writes nothing");
        std::process::exit(2);
    }
    if !o.removed.is_empty() && o.products.is_none() {
        eprintln!("--removed belongs to a products build (--products)");
        std::process::exit(2);
    }
    if o.products.is_some() {
        let jvm_build = o.command == "build" && o.jvm;
        if !(jvm_build || o.command == "check" && !o.interp) || o.output.is_some() || o.split.is_some() {
            eprintln!("--products belongs to `teq compiler build --target jvm` and to `teq compiler check` (a Scala.js module's check build), without -o or --split");
            std::process::exit(2);
        }
        // A module's products hold no launcher: every entry point gets scalac's class.
        o.all_mains = jvm_build;
    }
    if (!o.own.is_empty() || o.all_mains) && !o.jvm && !(o.check && !o.all_mains) && !(o.products.is_some() && o.command == "check") {
        eprintln!("--own and --all-mains belong to a JVM build (`teq compiler build` or `teq compiler watch` with --target jvm), --own to a check session as well");
        std::process::exit(2);
    }
    if let Some(v) = o.analysis_version {
        if !jvm::api::VERSIONS.contains(&v) {
            eprintln!("teq: analysis version {} is not one this teq answers: it answers versions 2 and 3", v);
            std::process::exit(2);
        }
        let answers = (o.command == "build" && o.jvm) || (o.command == "check" && !o.interp) || (o.command == "watch" && (o.jvm || o.check));
        if !answers {
            eprintln!("--analysis-version belongs to `teq compiler build --target jvm`, `teq compiler check` and `teq compiler watch` with --target jvm or --check");
            std::process::exit(2);
        }
    }
    o.std = match (o.jvm, std_given) {
        (true, Some(StdMode::Lean)) => {
            eprintln!("--std=lean is JavaScript's: a JVM build links against scala-library's bytecode and has no lean std; leave --std out or give --std=scala-library, with the scala-library jar on --classpath or in the coursier cache");
            exit_failed(&o, 2);
        }
        (true, _) => StdMode::ScalaLibrary,
        (false, given) => given.unwrap_or(StdMode::Lean),
    };
    if o.std == StdMode::ScalaLibrary {
        if !o.classpath.iter().any(|p| classpath::is_scala_library_jar(p)) {
            match classpath::find_scala_library() {
                Ok(jar) => o.classpath.insert(0, jar),
                Err(e) => {
                    eprintln!("{}", e);
                    exit_failed(&o, 2);
                }
            }
        }
    }
    // The project file sets the base, the command line adds to it.
    if let Some((path, text)) = dialect::Dialect::find_toml(&o.inputs) {
        match dialect::Dialect::from_toml(&text) {
            Ok(d) => o.dialect = d,
            Err(e) => {
                eprintln!("{}: {}", path, e);
                exit_failed(&o, 2);
            }
        }
        match dialect::max_inlines_from_toml(&text) {
            Ok(Some(n)) => o.max_inlines = n,
            Ok(None) => {}
            Err(e) => {
                eprintln!("{}: {}", path, e);
                exit_failed(&o, 2);
            }
        }
        match dialect::cacheable_state_from_toml(&text) {
            Ok(names) => o.cacheable_state.extend(names),
            Err(e) => {
                eprintln!("{}: {}", path, e);
                exit_failed(&o, 2);
            }
        }
    }
    let mut named = Vec::new();
    o.cacheable_state.retain(|n| {
        let new = !named.contains(n);
        named.push(n.clone());
        new
    });
    if let Some(n) = max_inlines {
        o.max_inlines = n;
    }
    // The module's own directory of products is read before the rest of the class path, and once.
    if let Some(i) = own_products_entry(&o) {
        let own = o.classpath.remove(i);
        let dir = classpath::canonical_lenient(std::path::Path::new(&own));
        o.classpath.retain(|p| classpath::canonical_lenient(std::path::Path::new(p)) != dir);
        o.classpath.insert(0, own);
    }
    for list in &flags {
        if let Err(e) = o.dialect.add_flags(list) {
            eprintln!("--dialect: {}", e);
            std::process::exit(2);
        }
    }
    o
}

/// The `.scala` files under `path`, or `path` itself: each with whether a directory listing
/// gave it, which spells it as the file system does.
fn collect_files(opts: &Options, path: &str, listed: bool, out: &mut Vec<(String, bool)>) {
    let p = std::path::Path::new(path);
    if p.is_dir() {
        let mut entries: Vec<_> = match std::fs::read_dir(p) {
            Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
            Err(e) => {
                eprintln!("cannot read {}: {}", path, e);
                exit_failed(opts, 2);
            }
        };
        entries.sort();
        for e in entries {
            let s = e.to_string_lossy().to_string();
            let hidden = e.file_name().map_or(false, |n| n.to_string_lossy().starts_with('.'));
            if s.ends_with(".scala") || (!hidden && e.is_dir()) {
                collect_files(opts, &s, true, out);
            }
        }
    } else {
        out.push((path.to_string(), listed));
    }
}

/// The `--profile` report on stderr, and its table as JSON where `json` names a file.
pub(crate) fn print_profile(typer: &mut typer::Worker, type_phase: std::time::Duration, json: Option<&str>) {
    eprint!("{}", typer.profile_report(type_phase, 20));
    eprint!("{}", typer.std_report());
    if let Some(path) = json {
        let table = typer.profile_json(type_phase);
        if let Err(e) = std::fs::write(path, table) {
            eprintln!("cannot write {}: {}", path, e);
        }
    }
}

/// The program's files: those of the inputs, in order, minus the excluded ones.
/// The program's files in the order of their identities (`program_keys`): a build's bytes do not
/// depend on the order the inputs are given in.
pub(crate) fn collect_program_paths(opts: &Options) -> Vec<String> {
    let mut paths = Vec::new();
    for input in &opts.inputs {
        collect_files(opts, input, false, &mut paths);
    }
    paths.retain(|(p, _)| !opts.excludes.iter().any(|x| p.starts_with(x.as_str()) || p.ends_with(x.as_str())));
    let paths = one_entry_per_file(paths);
    let keys = program_keys(opts, paths.iter().map(String::as_str));
    let mut order: Vec<usize> = (0..paths.len()).collect();
    order.sort_by(|&a, &b| keys[a].cmp(&keys[b]).then(paths[a].cmp(&paths[b])));
    order.into_iter().map(|i| paths[i].clone()).collect()
}

/// A file given twice, given with a directory that holds it, reached through a link or, on a
/// file system that ignores case, under another spelling of its case is one file of the build:
/// it stands once, where it first came, under the spelling a directory's listing gives it
/// (the file system's own) when one does, else under its path as the file system spells it,
/// the first of those by text.
fn one_entry_per_file(paths: Vec<(String, bool)>) -> Vec<String> {
    let mut seen: intern::FxMap<String, usize> = intern::FxMap::default();
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut spellings = Spellings::default();
    for (path, listed) in paths {
        let identity = file_identity(&path);
        let path = if listed { path } else { spellings.of(&path) };
        match seen.get(&identity) {
            Some(&i) => {
                let (kept, kept_listed) = &out[i];
                if (listed && !kept_listed) || (listed == *kept_listed && path < *kept) {
                    out[i] = (path, listed);
                }
            }
            None => {
                seen.insert(identity, out.len());
                out.push((path, listed));
            }
        }
    }
    out.into_iter().map(|(p, _)| p).collect()
}

/// What tells one file from another on the file system: its device and inode where the system
/// has them, else its resolved path.
fn file_identity(path: &str) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(m) = std::fs::metadata(path) {
            return format!("{}:{}", m.dev(), m.ino());
        }
    }
    source::canonicalize(path).map_or_else(|_| path.to_string(), |c| c.to_string_lossy().to_string())
}

/// The spelling of a file's path as the file system has it: each component of a path given by
/// hand replaced by the name its directory lists for the same file, where a file system that
/// ignores case gives the two names one file. Each directory is listed once.
#[derive(Default)]
struct Spellings {
    #[cfg(unix)]
    listings: intern::FxMap<std::path::PathBuf, intern::FxMap<(u64, u64), String>>,
}

impl Spellings {
    #[cfg(unix)]
    fn of(&mut self, path: &str) -> String {
        use std::os::unix::fs::MetadataExt;
        let mut spelled = std::path::PathBuf::new();
        for component in std::path::Path::new(path).components() {
            let std::path::Component::Normal(name) = component else {
                spelled.push(component.as_os_str());
                continue;
            };
            let candidate = spelled.join(name);
            let dir = if spelled.as_os_str().is_empty() { std::path::PathBuf::from(".") } else { spelled.clone() };
            let listed = self.listings.entry(dir.clone()).or_insert_with(|| {
                std::fs::read_dir(&dir).map(|rd| rd.filter_map(|e| e.ok()).filter_map(|e| e.metadata().ok().map(|m| ((m.dev(), m.ino()), e.file_name().to_string_lossy().to_string()))).collect()).unwrap_or_default()
            });
            match std::fs::metadata(&candidate).ok().and_then(|m| listed.get(&(m.dev(), m.ino()))) {
                Some(own) => spelled.push(own),
                None => spelled.push(name),
            }
        }
        spelled.to_string_lossy().to_string()
    }

    #[cfg(not(unix))]
    fn of(&mut self, path: &str) -> String {
        path.to_string()
    }
}

/// Per program file its identity, what the names made from its positions know it by
/// (`SourceFile::key`): the shortest suffix of its resolved path's components that no other
/// file of the build shares, joined by `/`, and never shorter than its path under the input
/// root it came from. A file under a root is known by that path (`com/example/Foo.scala`), a
/// file given as an input by its name (`Use.scala`), and where two files would go by one
/// identity each takes components of its path until they differ (`a/Use.scala` beside
/// `b/Use.scala`). The components are the resolved path's (links followed, the path absolute),
/// so `/src/Use.scala` and `src/Use.scala` of `/work` differ where they differ; nothing of the
/// machine enters a name while the suffix stays inside the project. A function of the set of
/// inputs alone, whatever order they are given in. Under `--sourceroot` a file under the root
/// is known by its path under it, the path its pickles name, so that a module built apart and
/// the whole program give a source one key.
pub(crate) fn program_keys<'p>(opts: &Options, paths: impl Iterator<Item = &'p str>) -> Vec<String> {
    let paths: Vec<&str> = paths.collect();
    let roots: Vec<(Vec<String>, Vec<String>)> = opts.inputs.iter().map(|i| (given_components(i), resolved_components(i))).collect();
    let root_of: intern::FxMap<&[String], usize> = roots.iter().enumerate().map(|(k, (given, _))| (given.as_slice(), k)).collect();
    let mut full: Vec<Vec<String>> = Vec::new();
    let mut take: Vec<usize> = Vec::new();
    for &path in &paths {
        let given = given_components(path);
        match (1..given.len()).rev().find_map(|k| root_of.get(&given[..k]).copied()) {
            Some(k) => {
                let (root, resolved) = &roots[k];
                let rel = &given[root.len()..];
                take.push(rel.len());
                full.push(resolved.iter().chain(rel.iter()).cloned().collect());
            }
            None => {
                let resolved = resolved_components(path);
                take.push(resolved.len().min(1));
                full.push(resolved);
            }
        }
    }
    let mut keys = shortest_unique_suffixes(&full, take);
    if opts.sourceroot.is_some() {
        let root = sourceroot(opts);
        let cwd = std::env::current_dir().ok();
        for (key, path) in keys.iter_mut().zip(&paths) {
            if let Some(rel) = tasty::write::path_under(path, &root, cwd.as_deref()) {
                *key = rel;
            }
        }
    }
    keys
}

/// The keys: each path's last `take` components, and while two paths give one key and differ
/// somewhere, one more component of each; two paths that agree throughout are one file.
fn shortest_unique_suffixes(paths: &[Vec<String>], mut take: Vec<usize>) -> Vec<String> {
    loop {
        let mut groups: intern::FxMap<String, Vec<usize>> = intern::FxMap::default();
        for (i, c) in paths.iter().enumerate() {
            groups.entry(c[c.len() - take[i]..].join("/")).or_default().push(i);
        }
        let mut grew = false;
        for members in groups.values().filter(|m| m.len() > 1) {
            for &i in members {
                if take[i] < paths[i].len() && members.iter().any(|&j| paths[j] != paths[i]) {
                    take[i] += 1;
                    grew = true;
                }
            }
        }
        if !grew {
            break;
        }
    }
    paths.iter().enumerate().map(|(i, c)| c[c.len() - take[i]..].join("/")).collect()
}

/// The named components of a path as given, `.` left out; a backslash separates components
/// on Windows alone.
fn given_components(path: &str) -> Vec<String> {
    std::path::Path::new(path)
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(n) => Some(n.to_string_lossy().to_string()),
            std::path::Component::ParentDir => Some("..".to_string()),
            _ => None,
        })
        .collect()
}

/// The named components of a path with its links followed and made absolute, or as given
/// where the file system has no such path (a text handed to a session).
fn resolved_components(path: &str) -> Vec<String> {
    let resolved = source::canonicalize(path).or_else(|_| std::path::absolute(path)).map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| path.to_string());
    given_components(&resolved)
}

pub(crate) fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Per file, whether a JVM build writes its class files: under `--own` the program files
/// under one of the roots (and the std's files), without it every file. `None` without `--own`.
pub(crate) fn owned_files(opts: &Options, files: &[SourceFile]) -> Option<Vec<bool>> {
    if opts.own.is_empty() {
        return None;
    }
    let under = |path: &str| opts.own.iter().any(|root| path == root || path.strip_prefix(root.as_str()).map_or(false, |rest| rest.starts_with(['/', '\\'])));
    Some(files.iter().map(|f| f.is_std || under(&f.path)).collect())
}

/// One empty source per jar of the classpath, and one for the JDK, which stand for them in
/// diagnostics and as the file of what is read from them.
pub(crate) fn jar_pseudo_files(opts: &Options, files: &mut Vec<SourceFile>, asts: &mut Vec<ast::Ast>) -> (Vec<source::FileId>, source::FileId) {
    let mut out = Vec::with_capacity(opts.classpath.len());
    for jar in opts.classpath.iter().map(String::as_str).chain(std::iter::once("<jdk>")) {
        out.push(source::FileId(files.len() as u32));
        files.push(SourceFile { path: jar.to_string(), text: String::new(), is_std: true, key: file_name(jar).to_string() });
        asts.push(ast::Ast::new(0));
    }
    let jdk = out.pop().unwrap();
    (out, jdk)
}

/// Opens the jars of `--classpath`; a jar that cannot be read ends the process. The JVM target
/// reaches the JDK's classes through the loader, with or without jars.
pub(crate) fn open_jars(opts: &Options) -> Option<classpath::Classpath> {
    try_open_jars(opts).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit_failed(opts, 2);
    })
}

/// The jars of the class path, or why one cannot be opened.
pub(crate) fn try_open_jars(opts: &Options) -> Result<Option<classpath::Classpath>, String> {
    if opts.classpath.is_empty() && !opts.jvm {
        return Ok(None);
    }
    let cache = if opts.no_cache { None } else { jarcache::dir() };
    let own = own_products_entry(opts).map(|i| classpath::OwnProducts {
        dir: classpath::canonical_lenient(std::path::Path::new(&opts.classpath[i])),
        dropped: collect_program_paths(opts).iter().chain(&opts.removed).map(|p| classpath::canonical_lenient(std::path::Path::new(p))).collect(),
    });
    classpath::Classpath::open(&opts.classpath, cache.as_deref(), own.as_ref()).map(Some)
}

/// The class path's entry that is the `--products` directory: a build of part of the module's
/// sources against its own products (docs/TARGETS.md, "The contract between the plugin and the
/// compiler").
fn own_products_entry(opts: &Options) -> Option<usize> {
    let dir = classpath::canonical_lenient(std::path::Path::new(opts.products.as_ref()?));
    opts.classpath.iter().position(|p| classpath::canonical_lenient(std::path::Path::new(p)) == dir)
}

/// Hands the typer the jars `open_jars` opened.
pub(crate) fn open_classpath(opts: &Options, typer: &mut typer::Worker, cp: Option<classpath::Classpath>, jar_files: Vec<source::FileId>, jdk_file: source::FileId) {
    if let Some(cp) = cp {
        typer.set_classpath(cp, jar_files, jdk_file, opts.jdk_release, opts.std);
    }
}

/// Writes the cache of the jars the build inflated entries of, for the next build.
fn save_jar_caches(typer: &mut typer::Worker) {
    if typer.loaded.is_some() {
        typer.loaded_mut().cp.save_caches();
    }
}

fn main() {
    argfile::init();
    if let Some(msg) = types::overlays_switch_error().or_else(typer::reach_mode_error) {
        eprintln!("{}", msg);
        std::process::exit(2);
    }
    types::view::invocation_begin();
    if std::env::var_os("TEQ_PANIC_REPORT").is_some_and(|v| v == "plain") {
        plain_panic_reports();
    }
    // Before the first request of 1 MB, which a resident process maps itself. `TEQ_RESIDENT=0`
    // keeps a session on the allocator of a batch run, for measuring what the exchange between
    // its threads costs and for the training of the profile-guided build (bench/pgo.sh).
    let mut words = argfile::args().iter();
    let resident = match words.next().map(String::as_str) {
        Some("lsp") => true,
        Some("compiler") => words.next().is_some_and(|verb| verb == "watch"),
        _ => false,
    };
    if resident {
        typer::thread::session();
    }
    if resident && !std::env::var_os("TEQ_RESIDENT").is_some_and(|v| v == "0") {
        alloc::resident();
    }
    // The typer recurses over expressions and through the definitions they name; a library's
    // bodies nest deeper than the default stack allows for.
    let compiler = std::thread::Builder::new().stack_size(1 << 30).spawn(compile).expect("cannot start the compiler thread");
    let code = compiler.join().unwrap_or(2);
    std::process::exit(code);
}

fn compile() -> i32 {
    alloc::enter();
    let opts = parse_options();
    if opts.command == "watch" {
        watch::run(&opts);
    }
    let t_start = Instant::now();
    answer_start(t_start);
    let mut program: Vec<SourceFile> = Vec::new();
    let paths = collect_program_paths(&opts);
    if opts.products.is_some() && paths.is_empty() {
        manifest_only(&opts);
    }
    let keys = program_keys(&opts, paths.iter().map(String::as_str));
    for (path, key) in paths.into_iter().zip(keys) {
        match std::fs::read_to_string(&path) {
            Ok(text) => program.push(SourceFile { path, text: planted::plant_clock(text, program.len()), is_std: false, key }),
            Err(e) => {
                eprintln!("cannot read {}: {}", path, e);
                answer_failure(&opts);
                std::process::exit(2);
            }
        }
    }
    let t_read = Instant::now();

    let mut interner = intern::Interner::new();
    let mut diags = Diagnostics::default();
    let lines: usize = program.iter().map(|f| f.text.lines().count()).sum();
    if opts.dump_tokens {
        for f in &program {
            for t in lexer::lex(&f.text, &mut interner).tokens {
                println!("{:?} {:?}", t.kind, &f.text[t.span.start as usize..t.span.end as usize]);
            }
        }
    }
    if opts.dump_defs {
        for f in &program {
            let (asts, errors) = frontend::parse_one(&f.text, &interner, opts.syntax);
            print!("file\t{}\n{}", f.path, parser::dump::dump(&asts, &errors, &interner));
        }
        std::process::exit(0);
    }
    // The jars open before parsing, which leaves out the std's files a jar defines, and count
    // with the classpath's other work in the type phase.
    let t_open = Instant::now();
    let cp = open_jars(&opts);
    let opening = t_open.elapsed();
    let program_bytes: usize = program.iter().map(|f| f.text.len()).sum();
    let (mut files, mut asts, blocks_of, std, selected, scalajs_unlocked) = parse_build(&opts, cp.as_ref(), program, &mut interner, &mut diags);
    let largest_unit = frontend::largest_unit_bytes(&files, &asts, &blocks_of);
    let (jar_files, jdk_file) = jar_pseudo_files(&opts, &mut files, &mut asts);
    let asts = Asts::new(asts);
    let files = Sources::new(files);
    let t_parse = Instant::now() - opening;
    // The recovered trees are typed whatever the parser reported, as scalac's typer runs; the
    // syntax errors come first, as scalac prints them. A parallel attempt that gave way printed
    // them before it typed (`serial_again`).
    let parse_errors = diags.error_count();
    if parse_errors > 0 && !serial_attempt() {
        eprint!("{}", diags.render(files.as_slice()));
    }

    let mut typer = typer::Typer::new(&asts, &files, &mut interner);
    typer.recovered = parse_errors > 0;
    typer.diags.syntax_errors = parse_errors > 0;
    // The build again by one worker, after a parallel attempt gave way (`serial_again`).
    let requested = frontend::requested_threads(opts.threads);
    typer.threads = if serial_attempt() { 1 } else { requested.unwrap_or_else(|| frontend::automatic_threads(program_bytes, largest_unit)) };
    install_std(&mut typer, std, &selected, &blocks_of, scalajs_unlocked);
    typer.profile.set_on(opts.profile);
    let p = typer.phase(typer::profile::Phase::Classpath);
    open_classpath(&opts, &mut typer, cp, jar_files, jdk_file);
    typer.phase_end(p);
    if let Some(main) = &opts.main {
        typer.set_main_name(main);
    }
    typer.cacheable_state = opts.cacheable_state.clone();
    typer.known_caches = !opts.no_known_caches;
    typer.macro_state_per_worker = opts.macro_state_per_worker;
    crate::measure::set_macro_state_per_worker(opts.macro_state_per_worker);
    typer.dialect = opts.dialect;
    if opts.wunused_imports {
        typer.unused.mode = typer::unused::Mode::Warn;
    }
    typer.inline.max_depth = opts.max_inlines;
    typer.inline.production = opts.release;
    typer.inline.es_modules = opts.split.is_some();
    // A module's products need no entry point: a Scala.js check's as a JVM build's (`all_mains`).
    typer.choose_entry = !opts.all_mains && opts.products.is_none();
    typer.open_world = opts.products.is_some() && opts.jvm;
    typer.writes_products = opts.products.is_some();
    // A Scala.js module's products record the expansions too, for a downstream to outline them
    // as the whole program does.
    typer.inline.outlines = (opts.command != "check" || opts.products.is_some()) && !opts.jvm && !opts.interp && !opts.no_outline;
    if opts.jvm {
        typer.set_jvm();
    }
    if opts.interp {
        typer.interp = true;
        typer.prog.record_types = true;
    }
    // A macro reads the types of the trees it is given, so a program with quotes, or with a
    // classpath whose libraries may define macros, keeps the type of every expression.
    if !opts.classpath.is_empty() || frontend::uses_quotes(&asts, files.as_slice()) {
        typer.prog.record_types = true;
    }
    // The product modes keep the typed form of their bodies and annotations for the pickles, an
    // analysis's API reads the annotations from them, and an answer with the dependencies reads
    // the bodies.
    if opts.products.is_some() || typer::capture::forced() || opts.analysis_version.is_some() {
        let owned = owned_files(&opts, typer.w.files.as_slice());
        typer.enable_capture(owned.as_deref());
    }
    if opts.analysis_version == Some(jvm::api::WITH_DEPS) {
        typer.deps = Some(Box::default());
    }
    measure::enable(opts.timings);
    measure::enable_parts(opts.timings);
    typer.run();
    typer.on_thread(|_| measure::flush(0));
    if typer::substitution_counts_on() {
        typer.on_thread(|w| w.count_retained_records());
    }
    measure::prep_measured(typer::prep::replayed().map(|r| r.kinds), typer::prep::write_capture());
    if typer.serial.is_needed() {
        let why = typer.serial.reason().unwrap_or_default();
        if requested.is_none() && !typer.serial.of_diagnostics() {
            let module = typer.serial.undeclared_module().filter(|(name, c)| typer.declared_by(name, *c)).map(|(name, _)| name);
            eprintln!("{}", give_way_note(&why, module.as_deref()));
        }
        return serial_again(&why, t_start.elapsed(), opts.timings);
    }
    // The merge's check refused the merged program: nothing reads it.
    if typer.merge_failed {
        eprint!("{}", typer.render_diags());
        let n = typer.diags.presented_error_count();
        eprintln!("{} error{} found", n, if n == 1 { "" } else { "s" });
        std::process::exit(1);
    }
    let t_capture = Instant::now();
    typer.on_thread(|w| w.finish_capture());
    if opts.timings {
        if let Some(c) = typer.prog.capture.as_deref() {
            eprintln!("capture: finished in {:.2} ms, {} records in {}, {} dropped with their typings", t_capture.elapsed().as_secs_f64() * 1000.0, c.records(), report::bytes(c.held() as u64), c.discarded);
        }
    }
    typer.on_thread(|w| w.capture_census());
    typer.flush_infos();
    planted::spin_type_phase(t_parse);
    let t_type = Instant::now();
    if opts.analysis_version.is_some() {
        answer_phase("read", t_read - t_start);
        answer_phase("classpath", opening);
        answer_phase("parse", t_parse - t_read);
        answer_phase("type", t_type - t_parse);
        let sources = typer.all_sources();
        answer_diagnostics(&[&diags, &typer.diags], &sources);
    }
    if std::env::var_os("TEQ_CLASSPATH_DETAIL").is_some() {
        typer.on_thread(|w| w.shape_census());
    }
    // A build with a classpath types library bodies in the reach phase, and an interpreted run
    // types the std's as it reaches them: their profiles wait.
    if opts.profile && !opts.interp && (opts.command == "check" || opts.classpath.is_empty()) {
        typer.on_thread(|w| print_profile(w, t_type - t_parse, opts.profile_json.as_deref()));
    }
    if !typer.diags.items.is_empty() {
        eprint!("{}", typer.render_diags());
    }
    if typer.diags.has_errors() || parse_errors > 0 {
        let n = typer.diags.presented_error_count() + parse_errors;
        eprintln!("{} error{} found", n, if n == 1 { "" } else { "s" });
        save_jar_caches(&mut typer);
        // The overlays' measurement reads a build that fails, which the part without the
        // crossings' imports can be.
        if opts.timings && typer::overlays_measured() {
            eprintln!("{}", loaded_report(&typer).render());
        }
        answer_failure(&opts);
        std::process::exit(1);
    }
    if opts.werror && typer.diags.warning_count() > 0 {
        let n = typer.diags.warning_count();
        eprintln!("{} warning{} found, errors under --werror", n, if n == 1 { "" } else { "s" });
        save_jar_caches(&mut typer);
        answer_failure(&opts);
        std::process::exit(1);
    }
    if opts.interp {
        save_jar_caches(&mut typer);
        return typer::thread::run(|| run_interp(&opts, typer.w, lines, [t_start, t_read, t_parse, t_type]));
    }
    if let Some(path) = std::env::var_os("TEQ_PRODUCT_INITS").filter(|_| !opts.jvm) {
        typer.on_thread(|w| dump_product_inits(w, &path.to_string_lossy()));
    }
    if let (Some(dir), "check") = (&opts.products, opts.command.as_str()) {
        let t_write = Instant::now();
        let written = pickle_products(&opts, &mut typer);
        let mut entries = product_entries(&typer.w, &written);
        check_inits(&typer.w, &mut entries);
        let files: Vec<(String, Vec<u8>)> = written.products.iter().map(|p| (products::tasty_path(p), p.bytes.clone())).collect();
        let own = typer.w.loaded.as_ref().and_then(|l| l.cp.own.as_ref());
        let (publication, generated) = products_publication(own, "js", Fresh { entries, files, std: Vec::new() }).unwrap_or_else(|e| {
            eprintln!("{}", e);
            exit_failed(&opts, 2)
        });
        save_jar_caches(&mut typer);
        if opts.timings {
            let mut report = loaded_report(&typer);
            let bytes: usize = written.products.iter().map(|p| p.bytes.len()).sum();
            let note = format!("{} TASTy files, {}", written.products.len(), report::bytes(bytes as u64));
            report.phases(lines, &[("read", t_read - t_start, ""), ("parse", t_parse - t_read, ""), ("type", t_type - t_parse, ""), ("tasty", t_write.elapsed(), note.as_str())], t_start.elapsed());
            eprintln!("{}", report.render());
        }
        let answer = opts.analysis_version.map(|_| {
            answer_phase("tasty", t_write.elapsed());
            let unit_file = unit_files(typer.w.files.len(), &blocks_of);
            answer_check_analysis(&opts, &mut typer, Some(&written.products), &unit_file, Some(&generated))
        });
        publish_products(&opts, dir, publication, answer);
        leave(0);
    }
    if opts.command == "check" {
        let total = t_start.elapsed();
        save_jar_caches(&mut typer);
        if opts.timings {
            let mut report = loaded_report(&typer);
            report.phases(lines, &[("read", t_read - t_start, ""), ("parse", t_parse - t_read, ""), ("type", t_type - t_parse, "")], total);
            eprintln!("{}", report.render());
        }
        if opts.analysis_version.is_some() {
            let unit_file = unit_files(typer.w.files.len(), &blocks_of);
            print_answer(Some(answer_check_analysis(&opts, &mut typer, None, &unit_file, None)));
        }
        leave(0);
    }

    if typer.prog.main.is_none() && typer.prog.js_exports.is_empty() && !(opts.all_mains && opts.command == "build") {
        save_jar_caches(&mut typer);
        eprintln!("error: the program has no entry point: an object or a package with `def main(args: Array[String]): Unit`, a `@main` method or an `@jsExport`");
        answer_failure(&opts);
        std::process::exit(1);
    }
    let array_seq = typer.on_thread(|w| w.array_seq_class());
    // The accessors of the inline bodies the products pickle, which the class files carry for a
    // scalac downstream that expands a body: made before the reach.
    if opts.jvm && opts.products.is_some() {
        typer.on_thread(|w| {
            let owned: Vec<bool> = w.prog.capture.as_deref().map(|c| c.owned.clone()).unwrap_or_default();
            w.add_inline_accessors(&|f| owned.get(f.0 as usize).copied().unwrap_or(false))
        });
    }
    let js_visible = opts.release && !opts.jvm;
    // The JVM's method names by `@targetName`, resolved before the reach: resolving an annotation
    // may enter its class, which the reach's tables then cover (`Worker::source_target_names`).
    let source_target_names = if opts.jvm { typer.on_thread(|w| w.source_target_names()) } else { intern::FxMap::default() };
    let t_compute = Instant::now();
    let mut reach = typer.on_thread(|w| emit::reach::compute(w, array_seq, js_visible));
    let t_reach = Instant::now();
    let t_computed = t_reach;
    if opts.profile && !opts.classpath.is_empty() {
        typer.on_thread(|w| print_profile(w, t_reach - t_parse, opts.profile_json.as_deref()));
    }
    typer.report_reached_library(&reach);
    // The bodies of the standard library and of the libraries on the classpath are typed as
    // the program reaches them.
    if typer.diags.has_errors() {
        save_jar_caches(&mut typer);
        eprint!("{}", typer.render_diags());
        if opts.timings || typer.loaded.as_ref().is_some_and(|loaded| !loaded.bodies.misses.is_empty()) {
            eprintln!("{}", loaded_report(&typer).render());
        }
        answer_failure(&opts);
        std::process::exit(1);
    }
    typer.on_thread(|_| measure::flush(0));
    // The writing counts with the reach, which read the jars; the report says what it took.
    let t_cache = Instant::now();
    save_jar_caches(&mut typer);
    let cache_time = t_cache.elapsed();
    let t_reach = t_reach + cache_time;
    let mut report = loaded_report(&typer);
    measure::report_parts(&mut report, measure::Pass::Reach, t_computed - t_compute, &[("before the walk", t_compute - t_type), ("jar caches written", cache_time)]);
    if opts.jvm {
        let array_seq = typer.on_thread(|w| w.array_seq_class());
        typer.rank_files();
        let products = opts.products.as_ref().map(|_| {
            let t = Instant::now();
            let written = pickle_products(&opts, &mut typer);
            (written, t.elapsed())
        });
        reach.cover(typer.w.syms.classes.len(), typer.w.syms.syms.len());
        let unit_file = unit_files(typer.w.files.len(), &blocks_of);
        build_jvm(&opts, typer.w, array_seq, &reach, &source_target_names, lines, [t_start, t_read, t_parse, t_type, t_reach], &asts, report, products, &unit_file);
    }
    let array_seq = typer.on_thread(|w| w.array_seq_class());
    typer.rank_files();
    let collisions = typer.product_name_collisions(&reach.classes);
    if !collisions.is_empty() {
        for c in &collisions {
            eprintln!("error: {}", c);
        }
        eprintln!("{} error{} found", collisions.len(), if collisions.len() == 1 { "" } else { "s" });
        answer_failure(&opts);
        std::process::exit(1);
    }
    let all_sources = typer.all_sources();
    let typer::Worker { prog, syms, file_pkgs, .. } = typer.w;
    let split = opts
        .split
        .as_ref()
        .map(|_| emit::Split { files: files.as_slice(), file_pkgs: file_pkgs.own(), module_per_file: &opts.module_per_file, hot: opts.hot });
    let options = emit::Options { release: opts.release, size_report: opts.size_report.then(|| all_sources.as_slice()), outline: !opts.no_outline };
    let t_emit_start = Instant::now();
    let (mut output, size_report) = emit::emit(&prog, &syms, &interner, array_seq, &reach, split, options, None);
    let t_emit = Instant::now();
    measure::report_parts(&mut report, measure::Pass::Emit, t_emit - t_emit_start, &[("before the emit", (t_emit_start - t_computed).saturating_sub(cache_time))]);

    let written = match (&mut output, &opts.split) {
        (emit::Output::Modules(modules), Some(dir)) => {
            let written = write::modules(dir, modules).unwrap_or_else(|e| {
                eprintln!("cannot write {}: {}", dir, e);
                exit_with_report(&opts, &report, 2);
            });
            format!("{} of {} modules", written.len(), modules.files.len())
        }
        (emit::Output::Script(js), _) => {
            // node decides between script and ES module by the extension.
            let default_path = if prog.is_module() { "out/main.mjs" } else { "out/main.js" };
            let out_path = opts.output.clone().unwrap_or_else(|| default_path.to_string());
            if let Err(e) = write::file(&out_path, js) {
                eprintln!("cannot write {}: {}", out_path, e);
                exit_with_report(&opts, &report, 2);
            }
            String::new()
        }
        (emit::Output::Modules(_), None) => unreachable!(),
    };
    let t_write = Instant::now();
    if let Some(report) = size_report {
        eprint!("{}", report);
    }
    if opts.timings {
        let phases = [
            ("read", t_read - t_start, ""),
            ("parse", t_parse - t_read, ""),
            ("type", t_type - t_parse, ""),
            ("reach", t_reach - t_type, ""),
            ("emit", t_emit - t_reach, ""),
            ("write", t_write - t_emit, written.as_str()),
        ];
        report.phases(lines, &phases, t_write - t_start);
        eprintln!("{}", report.render());
    }
    leave(0)
}

/// Ends a run that is done without dropping what it built: the program's blocks would be
/// freed one by one, 0.4 to 1.3% of a check's instructions, for memory the process gives back
/// as a whole.
fn leave(code: i32) -> ! {
    std::process::exit(code)
}

/// What the pickles' source paths are relative to: `--sourceroot`, or the working directory.
pub(crate) fn sourceroot(opts: &Options) -> std::path::PathBuf {
    let root = opts.sourceroot.clone().map(std::path::PathBuf::from).unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    if root.is_absolute() {
        root
    } else {
        std::env::current_dir().unwrap_or_default().join(root)
    }
}

/// Under `--analysis-version`, the answer of a one-shot build that failed: `ok` false and no
/// analysis, on stdout, with the diagnostics and the phases' times known by then; the
/// diagnostics are on stderr as for any build.
fn answer_failure(opts: &Options) {
    if let Some(v) = opts.analysis_version {
        use std::io::Write as _;
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{{\"ok\":false,\"analysisVersion\":{}{}}}", v, answer_extras());
        let _ = out.flush();
    }
}

/// What a one-shot answer under `--analysis-version` carries besides the analysis, gathered as
/// the build goes: its diagnostics as a session's answer gives them (`watch::render_check`),
/// once the program is typed, and the time of each phase in milliseconds.
struct AnswerExtras {
    diagnostics: Option<String>,
    ms: Vec<(&'static str, f64)>,
    start: Option<Instant>,
}

static ANSWER_EXTRAS: std::sync::Mutex<AnswerExtras> = std::sync::Mutex::new(AnswerExtras { diagnostics: None, ms: Vec::new(), start: None });

fn answer_start(t: Instant) {
    ANSWER_EXTRAS.lock().unwrap_or_else(|e| e.into_inner()).start = Some(t);
}

fn answer_phase(name: &'static str, d: Duration) {
    ANSWER_EXTRAS.lock().unwrap_or_else(|e| e.into_inner()).ms.push((name, d.as_secs_f64() * 1000.0));
}

fn answer_diagnostics(sets: &[&source::Diagnostics], files: &[source::SourceFile]) {
    let mut items: Vec<String> = Vec::new();
    for d in sets {
        items.extend(watch::render_check(d, files));
    }
    ANSWER_EXTRAS.lock().unwrap_or_else(|e| e.into_inner()).diagnostics = Some(format!("[{}]", items.join(",")));
}

/// `,"diagnostics":[...],"ms":{...}` of what is known, `total` the time since the build began.
fn answer_extras() -> String {
    use std::fmt::Write as _;
    let extras = ANSWER_EXTRAS.lock().unwrap_or_else(|e| e.into_inner());
    let mut out = String::new();
    if let Some(d) = &extras.diagnostics {
        let _ = write!(out, ",\"diagnostics\":{}", d);
    }
    out.push_str(",\"ms\":{");
    for (i, (name, ms)) in extras.ms.iter().enumerate() {
        let _ = write!(out, "{}\"{}\":{:.3}", if i > 0 { "," } else { "" }, name, ms);
    }
    if let Some(start) = extras.start {
        let _ = write!(out, "{}\"total\":{:.3}", if extras.ms.is_empty() { "" } else { "," }, start.elapsed().as_secs_f64() * 1000.0);
    }
    out.push('}');
    out
}

/// Ends a build that failed before its answer, with the failure answer under `--analysis-version`.
fn exit_failed(opts: &Options, code: i32) -> ! {
    answer_failure(opts);
    std::process::exit(code)
}

/// The file each unit's classes are reported under: a `package p:` block's is its file's.
fn unit_files(n: usize, blocks_of: &[Vec<usize>]) -> Vec<source::FileId> {
    let mut unit_file: Vec<source::FileId> = (0..n).map(|i| source::FileId(i as u32)).collect();
    for (file, blocks) in blocks_of.iter().enumerate() {
        for &b in blocks {
            if b < n {
                unit_file[b] = source::FileId(file as u32);
            }
        }
    }
    unit_file
}

/// The files a one-shot build's analysis covers: the program's, `--own`'s where it is given.
fn analysis_files(opts: &Options, w: &typer::Worker) -> Vec<source::FileId> {
    let all = w.all_sources();
    let owned = owned_files(opts, &all);
    (0..w.files.len())
        .map(|i| source::FileId(i as u32))
        .filter(|f| {
            let i = f.0 as usize;
            !all[i].is_std && !w.in_jar(*f) && owned.as_ref().map_or(true, |o| o[i])
        })
        .collect()
}

/// The analysis of a one-shot check: the classes of the files for discovery, and the graph.
fn answer_check_analysis(opts: &Options, typer: &mut typer::Typer, pickled: Option<&[tasty::write::Product]>, unit_file: &[source::FileId], generated: Option<&str>) -> String {
    let files = analysis_files(opts, &typer.w);
    let analysis = typer.on_thread(|w| jvm::analysis::render_typed(w, &files, unit_file));
    typer.on_thread(|w| answer_analysis(opts, w, analysis, &files, unit_file, pickled, generated))
}

/// A one-shot build's analysis as one JSON line, `{"ok":true,"analysisVersion":2,
/// "analysis":[...],"api":[...]}`, with a products build's `generated` journal; a definition the
/// pickler cannot write, or a type the graph cannot state, fails the build.
fn answer_analysis(opts: &Options, w: &mut typer::Worker, analysis: Vec<String>, files: &[source::FileId], unit_file: &[source::FileId], pickled: Option<&[tasty::write::Product]>, generated: Option<&str>) -> String {
    let root = sourceroot(opts);
    let start = Instant::now();
    let version = opts.analysis_version.unwrap_or(2);
    let api = match jvm::api::render(w, files, unit_file, pickled, &root, version) {
        Ok(api) => api,
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            eprintln!("the analysis graph cannot state {} construct{}", errors.len(), if errors.len() == 1 { "" } else { "s" });
            answer_failure(opts);
            std::process::exit(1);
        }
    };
    let rendered = start.elapsed();
    answer_phase("api", rendered);
    let generated = generated.map(|g| format!(",\"generated\":{}", g)).unwrap_or_default();
    let line = format!("{{\"ok\":true,\"analysisVersion\":{},\"analysis\":[{}],\"api\":{}{}{}}}\n", version, analysis.join(","), api, generated, answer_extras());
    if opts.timings {
        eprintln!("analysis: graph of {} files in {:.2} ms ({} of the graph, {} in all), written in {:.2} ms", files.len(), rendered.as_secs_f64() * 1000.0, report::bytes(api.len() as u64), report::bytes(line.len() as u64), (start.elapsed() - rendered).as_secs_f64() * 1000.0);
        if version >= 3 {
            let deps = typer::deps::RENDER_NANOS.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e6;
            eprintln!("analysis: dependencies in {:.2} ms of the graph's", deps);
        }
    }
    line
}

/// Whether this process is the one-worker build a parallel attempt gave way to.
fn serial_attempt() -> bool {
    std::env::var_os("TEQ_SERIAL").is_some()
}

/// The parallel attempt this process's build gave way to: its time and its reason
/// (`serial_again`), for the `--time` report.
fn serial_cause() -> Option<(Duration, String)> {
    let v = std::env::var("TEQ_SERIAL").ok()?;
    let (ms, why) = v.split_once(';')?;
    Some((Duration::from_secs_f64(ms.parse::<f64>().ok()? / 1000.0), why.to_string()))
}

/// What a build whose worker count was automatic says when its parallel attempt gave way: why,
/// and what keeps the next build on its workers or types it by one worker from the start.
pub(crate) fn give_way_note(why: &str, module: Option<&str>) -> String {
    let declare = match module {
        Some(m) => format!("; if {m} holds nothing but a cache whose contents do not change what a macro makes, --cacheable-state {m} (teqCacheableState in sbt, cacheableState in a build description) declares it, and its changes no longer give the build away"),
        None => String::new(),
    };
    format!("teq: a parallel attempt gave way to one worker: {why}{declare}; --threads 1 (teqThreads in sbt, threads in a build description) types the build by one worker from the start")
}

/// The build again from the start by one worker, in place of this process, whose parallel
/// attempt met an outcome the workers' order decides (`typer::Worker::need_serial`): nothing
/// it made or printed stands, and what one worker's build makes is scalac's order by
/// construction. The process image is replaced,
/// so the attempt's memory goes with it (on Windows, which has no exec, once the build ends:
/// `hand_over`).
fn serial_again(why: &str, attempt: Duration, timings: bool) -> i32 {
    types::view::gave_way(why);
    if std::env::var_os("TEQ_SERIAL_TRACE").is_some() {
        eprintln!("teq: typed again by one worker: {}", why);
    }
    // What the attempt measured goes with its process image: its section is printed first.
    measure::write_files(".attempt");
    if timings {
        let mut report = report::Report::default();
        measure::report(&mut report, true, |_| {});
        eprintln!("{}", report.render());
    }
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            eprintln!("teq: cannot find this program to type the build again by one worker: {}", e);
            return 2;
        }
    };
    let cause = format!("{:.1};{}", attempt.as_secs_f64() * 1000.0, why);
    // The arguments as this process received them, an argument file's `@<path>` unexpanded, so
    // that the command stays under Windows' cap; its writer keeps the file for this process's life.
    let mut command = std::process::Command::new(exe);
    command.args(std::env::args_os().skip(1)).env("TEQ_SERIAL", cause);
    let err = hand_over(command);
    eprintln!("teq: cannot type the build again by one worker: {}", err);
    2
}

/// This process replaced by the command's, or the error that kept it.
#[cfg(unix)]
fn hand_over(mut command: std::process::Command) -> std::io::Error {
    use std::os::unix::process::CommandExt;
    command.exec()
}

/// Without an exec the command runs as a child and this process ends with its exit code, all 32
/// bits of it, once the child ends; until then the attempt's memory stays this process's. The
/// console delivers Ctrl-C and Ctrl-Break to both: this process takes them as handled, as qbt's
/// launcher does, so that it outlives the child to pass the code on. A handler, unlike ignoring
/// the events, is not inherited by the child.
#[cfg(windows)]
fn hand_over(mut command: std::process::Command) -> std::io::Error {
    const CTRL_C_EVENT: u32 = 0;
    const CTRL_BREAK_EVENT: u32 = 1;
    unsafe extern "system" fn handled(event: u32) -> i32 {
        (event == CTRL_C_EVENT || event == CTRL_BREAK_EVENT) as i32
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn SetConsoleCtrlHandler(handler: Option<unsafe extern "system" fn(u32) -> i32>, add: i32) -> i32;
    }
    // SAFETY: registers a handler that reads its argument alone.
    unsafe { SetConsoleCtrlHandler(Some(handled), 1) };
    match command.status() {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(err) => err,
    }
}

/// What the build read from the jars, as the first sections of its `--time` report.
fn loaded_report(typer: &typer::Typer) -> report::Report {
    let mut report = report::Report::default();
    measure::write_files("");
    let cause = serial_cause();
    if cause.is_some() || typer.threads > 1 || measure::section_wanted() {
        measure::report(&mut report, false, |section| {
            if let Some((attempt, why)) = cause {
                section.row("parallel attempt").time(attempt).note(format!("gave way to one worker: {}", why));
            }
        });
    }
    if let Some(loaded) = &typer.loaded {
        loaded.report(&mut report, |c| typer.class_path(c));
    }
    report
}

/// Leaves a build that failed after its jars were read, with what `--time` has of them.
fn exit_with_report(opts: &Options, report: &report::Report, code: i32) -> ! {
    if opts.timings && !report.is_empty() {
        eprintln!("{}", report.render());
    }
    std::process::exit(code)
}

#[allow(clippy::too_many_arguments)]
fn build_jvm(
    opts: &Options,
    mut typer: typer::Worker,
    array_seq: Option<types::ClassId>,
    reach: &emit::reach::Reach,
    source_target_names: &intern::FxMap<types::SymId, intern::Name>,
    lines: usize,
    t: [Instant; 5],
    asts: &Asts,
    mut report: report::Report,
    products: Option<(tasty::write::Written, Duration)>,
    unit_file: &[source::FileId],
) -> ! {
    let all_sources = typer.all_sources();
    let pickled: Vec<Pickled> = products
        .as_ref()
        .map(|(w, _)| {
            w.products
                .iter()
                .map(|p| Pickled { package: p.package.clone(), name: p.name.clone(), uuid: p.uuid, source: all_sources[p.source.0 as usize].path.clone(), complete: p.complete })
                .collect()
        })
        .unwrap_or_default();
    let owned = owned_files(opts, &all_sources);
    let stackable = stackable_overrides(&typer, reach);
    let typer::Worker { prog, syms, file_pkgs, types, tvars, interner, loaded, entry_points, mixin_supers, jvm_volatile, .. } = &mut typer;
    let product_inits = loaded.as_ref().map(|l| l.get_mut().cp.product_inits.clone()).unwrap_or_default();
    // A JVM build always reads the class path: scala-library is on it.
    let typer::loader::Loaded { cp, classes, java, target_names, product_target_names, volatile, .. } = loaded.as_ref().expect("a JVM build reads its class path").get_mut();
    let input = jvm::Input {
        prog: &*prog,
        syms: &*syms,
        types: &*types,
        insts: &*tvars,
        interner: &**interner,
        reach,
        array_seq,
        sources: &all_sources,
        file_pkgs: file_pkgs.own(),
        asts,
        java: Some(&*java),
        target_names: Some(&*target_names),
        source_target_names: Some(source_target_names),
        product_target_names: Some(&*product_target_names),
        volatile: Some(&*volatile),
        source_volatile: Some(&*jvm_volatile),
        link: jvm::Link { jar_classes: &*classes },
        owned: owned.as_deref(),
        all_mains: opts.all_mains.then_some(entry_points.as_slice()),
        output_version: opts.java_output_version,
        open_world: opts.products.is_some(),
        mixin_supers: opts.products.is_some().then_some(&*mixin_supers),
        stackable: Some(&stackable),
        product_inits: Some(&product_inits),
    };
    let t_emit_start = Instant::now();
    let output = jvm::emit(input, cp, None);
    let t_emit = Instant::now();
    measure::report_parts(&mut report, measure::Pass::Emit, t_emit - t_emit_start, &[]);
    if !output.errors.is_empty() {
        for e in &output.errors {
            eprintln!("jvm backend: {}", e);
        }
        eprintln!("{} construct{} not supported by the JVM backend", output.errors.len(), if output.errors.len() == 1 { "" } else { "s" });
        answer_failure(opts);
        exit_with_report(opts, &report, 3);
    }
    let mut output = output;
    let (out_path, publication) = match &opts.products {
        Some(dir) => {
            let owned: Vec<bool> = all_sources.iter().map(|s| !s.is_std).collect();
            let entries = mark_products(&mut output, &pickled, &all_sources, &owned);
            let pickles = products.as_ref().map_or(&[][..], |(w, _)| w.products.as_slice());
            let fresh = jvm_fresh_products(&output, entries, &all_sources, &owned, pickles);
            let own = typer.loaded.as_ref().and_then(|l| l.cp.own.as_ref());
            let publication = products_publication(own, "jvm", fresh).unwrap_or_else(|e| {
                eprintln!("{}", e);
                answer_failure(opts);
                exit_with_report(opts, &report, 2)
            });
            (dir.clone(), Some(publication))
        }
        None => {
            let out_path = opts.output.clone().unwrap_or_else(|| "out/main.jar".to_string());
            if let Err(e) = jvm::write(&out_path, &output) {
                eprintln!("cannot write {}: {}", out_path, e);
                answer_failure(opts);
                exit_with_report(opts, &report, 2);
            }
            (out_path, None)
        }
    };
    let answer = opts.analysis_version.map(|_| {
        let tasty_time = products.as_ref().map_or(Duration::ZERO, |(_, d)| *d);
        answer_phase("reach", t[4] - t[3]);
        answer_phase("tasty", tasty_time);
        answer_phase("emit", (t_emit - t[4]).saturating_sub(tasty_time));
        let files = analysis_files(opts, &typer);
        let analysis = jvm::analysis::render(&mut typer, &output, &files, unit_file);
        let pickled = products.as_ref().map(|(w, _)| w.products.as_slice());
        answer_analysis(opts, &mut typer, analysis, &files, unit_file, pickled, publication.as_ref().map(|(_, g)| g.as_str()))
    });
    match publication {
        Some((p, _)) => publish_products(opts, &out_path, p, answer),
        None => print_answer(answer),
    }
    let t_write = Instant::now();
    if opts.timings {
        let bytes: usize = output.classes.iter().map(|c| c.bytes.len()).sum();
        let written = format!("{} classes, {}", report::grouped(output.classes.len() as u64), report::bytes(bytes as u64));
        let (tasty_time, tasty_note) = match &products {
            Some((w, d)) => {
                let b: usize = w.products.iter().map(|p| p.bytes.len()).sum();
                (*d, format!("{} TASTy files, {}", w.products.len(), report::bytes(b as u64)))
            }
            None => (Duration::ZERO, String::new()),
        };
        let mut phases = vec![
            ("read", t[1] - t[0], ""),
            ("parse", t[2] - t[1], ""),
            ("type", t[3] - t[2], ""),
            ("reach", t[4] - t[3], ""),
        ];
        if products.is_some() {
            phases.push(("tasty", tasty_time, tasty_note.as_str()));
        }
        phases.push(("emit", t_emit - t[4] - tasty_time, ""));
        phases.push(("write", t_write - t_emit, written.as_str()));
        report.phases(lines, &phases, t_write - t[0]);
        eprintln!("{}", report.render());
    }
    leave(0)
}

/// The TASTy files of the program's owned files, for the products' publication; a definition
/// the writer cannot write fails the build with the construct named.
fn pickle_products(opts: &Options, typer: &mut typer::Typer) -> tasty::write::Written {
    // The accessors the inline bodies read private members through, which the pickles hold,
    // and the results of the inline methods declared without one.
    typer.on_thread(|w| {
        let owned: Vec<bool> = w.prog.capture.as_deref().map(|c| c.owned.clone()).unwrap_or_default();
        w.add_inline_accessors(&|f| owned.get(f.0 as usize).copied().unwrap_or(false));
        w.infer_inline_results(&|f| owned.get(f.0 as usize).copied().unwrap_or(false))
    });
    let root = sourceroot(opts);
    let owned = owned_files(opts, typer.w.files.as_slice());
    if let Some(path) = std::env::var_os("TEQ_STD_SHAPES") {
        let keys = typer.on_thread(|w| {
            w.enter_all_std();
            tasty::write::std_shapes(w, &root)
        });
        let text: String = keys.iter().map(|k| format!("{}\n", k)).collect();
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("{}: {}", std::path::Path::new(&path).display(), e);
            leave(1);
        }
        leave(0);
    }
    let t_pickle = Instant::now();
    let written = typer.on_thread(|w| tasty::write::write_products(w, owned.as_deref(), &root));
    let pickled = t_pickle.elapsed();
    if !written.errors.is_empty() {
        for e in &written.errors {
            eprintln!("{}", e);
        }
        eprintln!("{} definition{} cannot be written as TASTy", written.errors.len(), if written.errors.len() == 1 { "" } else { "s" });
        answer_failure(opts);
        std::process::exit(1);
    }
    if let Some(path) = std::env::var_os("TEQ_BODIES_CENSUS") {
        let mut rows: Vec<(&String, &u32)> = written.bodies.iter().collect();
        rows.sort();
        let text: String = rows.iter().map(|(k, v)| format!("body\t{}\t{}\n", k, v)).collect();
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(text.as_bytes());
        }
    }
    if std::env::var_os("TEQ_PRODUCTS_DETAIL").is_some() {
        let mut shapes: Vec<(&String, &u32)> = written.approximated.iter().collect();
        shapes.sort();
        for (k, v) in shapes {
            eprintln!("approximated: {} ({})", k, v);
        }
    }
    if opts.timings {
        eprintln!("tasty: pickled in {:.2} ms", pickled.as_secs_f64() * 1000.0);
    }
    written
}

/// The `abstract override` members of the traits of the class path's product directories.
pub(crate) fn stackable_overrides(typer: &typer::Worker, reach: &emit::reach::Reach) -> intern::FxMap<types::SymId, ()> {
    let mut out = intern::FxMap::default();
    let Some(loaded) = typer.loaded.as_ref() else { return out };
    for (i, &product) in reach.product_classes.iter().enumerate() {
        let info = typer.syms.class(types::ClassId(i as u32));
        if !product || info.kind != symbols::ClassKind::Trait {
            continue;
        }
        let both = ast::mods::ABSTRACT | ast::mods::OVERRIDE;
        for &s in &info.member_order {
            if typer.syms.sym(s).mods & both == both && loaded.has_body(s) {
                out.insert(s, ());
            }
        }
    }
    out
}

/// A pickle of the build, with the source that owns it.
struct Pickled {
    package: String,
    name: String,
    uuid: [u8; 16],
    source: String,
    /// Every body written.
    complete: bool,
}

/// The files of a JVM products build: its `.tasty` and class files and the std classes it wrote,
/// each entry with its objects that initialise and the std classes its class files name, closed
/// over the names the std classes hold in turn.
fn jvm_fresh_products(output: &jvm::Output, mut entries: Vec<products::Entry>, sources: &[SourceFile], owned: &[bool], pickles: &[tasty::write::Product]) -> Fresh {
    let is_std = |c: &jvm::Emitted| c.source.map_or(true, |f| sources[f.0 as usize].is_std);
    let std_names: intern::FxMap<&str, ()> = output.classes.iter().filter(|c| is_std(c)).map(|c| (c.name.as_str(), ())).collect();
    let bytes: intern::FxMap<String, &[u8]> = output.classes.iter().map(|c| (format!("{}.class", c.name), c.bytes.as_slice())).collect();
    let std_refs: intern::FxMap<&str, Vec<&str>> = output.classes.iter().filter(|c| is_std(c)).map(|c| (c.name.as_str(), products::classes_named(&c.bytes, &std_names))).collect();
    let owned_classes: intern::FxMap<&str, ()> = output.classes.iter().filter(|c| c.source.map_or(false, |f| owned[f.0 as usize])).map(|c| (c.name.as_str(), ())).collect();
    let inits: Vec<&String> = output.inits.iter().filter(|n| owned_classes.contains_key(n.as_str())).collect();
    for e in entries.iter_mut() {
        let mut named: Vec<&str> = Vec::new();
        let mut seen: intern::FxMap<&str, ()> = intern::FxMap::default();
        for b in e.classes.iter().filter_map(|c| bytes.get(c)) {
            for n in products::classes_named(b, &std_names) {
                if seen.insert(n, ()).is_none() {
                    named.push(n);
                }
            }
        }
        let mut i = 0;
        while i < named.len() {
            for &n in std_refs.get(named[i]).into_iter().flatten() {
                if seen.insert(n, ()).is_none() {
                    named.push(n);
                }
            }
            i += 1;
        }
        e.std = named.iter().map(|n| format!("{}.class", n)).collect();
        e.std.sort();
        let classes: intern::FxMap<&str, ()> = e.classes.iter().map(|c| (c.as_str(), ())).collect();
        e.inits = inits.iter().filter(|n| classes.contains_key(format!("{}.class", n).as_str())).map(|n| n.to_string()).collect();
    }
    let mut files: Vec<(String, Vec<u8>)> = pickles.iter().map(|p| (products::tasty_path(p), p.bytes.clone())).collect();
    for e in &entries {
        files.extend(e.classes.iter().filter_map(|c| bytes.get(c).map(|b| (c.clone(), b.to_vec()))));
    }
    let std = output.classes.iter().filter(|c| is_std(c)).map(|c| (format!("{}.class", c.name), c.bytes.to_vec())).collect();
    Fresh { entries, files, std }
}

/// The manifest's entries of a JVM build, each class file under the product its name falls
/// under, and the attributes `GenBCode` writes: `Scala` on every class of an owned source,
/// `TASTY` with the product's UUID on the class that holds its pickle.
fn mark_products(
    output: &mut jvm::Output,
    products: &[Pickled],
    sources: &[SourceFile],
    owned: &[bool],
) -> Vec<products::Entry> {
    // A product is owned by the source it was pickled from, whether or not it has class files
    // (a file of type aliases alone has none).
    // Each owning source's fingerprint once, whatever number of pickles it has.
    let mut fingerprints: std::collections::HashMap<&str, Option<String>> = std::collections::HashMap::new();
    for p in products {
        if !fingerprints.contains_key(p.source.as_str()) {
            let text = sources.iter().find(|s| s.path == p.source).map(|s| products::fingerprint(&s.text));
            fingerprints.insert(p.source.as_str(), text);
        }
    }
    let fingerprint = |path: &str| fingerprints.get(path).cloned().flatten();
    let mut entries: Vec<products::Entry> = products
        .iter()
        .map(|p| products::Entry {
            source: p.source.clone(),
            fingerprint: fingerprint(&p.source),
            tasty: Some(if p.package.is_empty() { format!("{}.tasty", p.name) } else { format!("{}/{}.tasty", p.package, p.name) }),
            classes: Vec::new(),
            bodies: p.complete,
            inits: Vec::new(),
            std: Vec::new(),
            digest: String::new(),
        })
        .collect();
    // The pickle's class, as `GenBCode` places the attribute: the class of the product's name,
    // else (an object without a companion class, whose mirror teq does not write) its module
    // class.
    let holders: Vec<String> = products
        .iter()
        .map(|p| {
            let plain = if p.package.is_empty() { p.name.clone() } else { format!("{}/{}", p.package, p.name) };
            if output.classes.iter().any(|c| c.name == plain) { plain } else { format!("{}$", plain) }
        })
        .collect();
    let mut extra: Vec<products::Entry> = Vec::new();
    for c in output.classes.iter_mut() {
        let Some(f) = c.source else { continue };
        if !owned.get(f.0 as usize).copied().unwrap_or(false) {
            continue;
        }
        let (pkg, stem) = match c.name.rsplit_once('/') {
            Some((p, s)) => (p, s),
            None => ("", c.name.as_str()),
        };
        // A class of another source than the product's (a quote's copy made at an expansion's
        // site, whose name is the quote's file's) is that source's, as a downstream module that
        // makes it lists it.
        let own = sources[f.0 as usize].path.as_str();
        let best = products
            .iter()
            .enumerate()
            .filter(|(_, p)| p.package == pkg && p.source == own && (stem == p.name || stem.starts_with(&format!("{}$", p.name))))
            .max_by_key(|(_, p)| p.name.len())
            .map(|(i, _)| i);
        let mut attrs: Vec<(&str, Vec<u8>)> = Vec::new();
        match best {
            Some(i) => {
                entries[i].classes.push(format!("{}.class", c.name));
                if holders[i] == c.name {
                    attrs.push(("TASTY", products[i].uuid.to_vec()));
                }
            }
            None => extra.push(products::Entry {
                source: sources[f.0 as usize].path.clone(),
                fingerprint: Some(products::fingerprint(&sources[f.0 as usize].text)),
                tasty: None,
                classes: vec![format!("{}.class", c.name)],
                bodies: false,
                inits: Vec::new(),
                std: Vec::new(),
                digest: String::new(),
            }),
        }
        attrs.push(("Scala", Vec::new()));
        let refs: Vec<(&str, &[u8])> = attrs.iter().map(|(n, b)| (*n, b.as_slice())).collect();
        match products::add_class_attributes(&c.bytes, &refs) {
            Ok(b) => c.bytes = b.into(),
            Err(e) => eprintln!("{}: {}", c.name, e),
        }
    }
    for e in entries.iter_mut() {
        e.classes.sort();
    }
    entries.extend(extra);
    entries
}

fn product_entries(typer: &typer::Worker, written: &tasty::write::Written) -> Vec<products::Entry> {
    let paths: Vec<String> = typer.files.as_slice().iter().map(|f| f.path.clone()).collect();
    let mut fingerprints: std::collections::HashMap<crate::source::FileId, String> = std::collections::HashMap::new();
    written
        .products
        .iter()
        .map(|p| products::Entry {
            source: paths[p.source.0 as usize].clone(),
            fingerprint: Some(fingerprints.entry(p.source).or_insert_with(|| products::fingerprint(&typer.files.as_slice()[p.source.0 as usize].text)).clone()),
            tasty: Some(products::tasty_path(p)),
            classes: Vec::new(),
            bodies: p.complete,
            inits: Vec::new(),
            std: Vec::new(),
            digest: String::new(),
        })
        .collect()
}

/// Under `TEQ_PRODUCT_INITS=<file>`, a build over product directories on JavaScript or in the
/// interpreter converts and checks every object of them and appends, per directory, the binary
/// names of those whose converted body runs code at construction, by the rule the manifests'
/// `inits` are written by: what the module harness holds both manifests to.
fn dump_product_inits(typer: &mut typer::Worker, path: &str) {
    let objects = typer.product_objects();
    for &(_, c) in &objects {
        typer.check_library_class(c);
    }
    let layout = emit::layout::Layout::new(&typer.prog, &typer.syms, typer.interner);
    let mut lines: Vec<String> = Vec::new();
    for (dir, c) in objects {
        if let Some(name) = products::object_inits(&typer.syms, typer.interner, &layout, std::iter::once(c)).pop() {
            lines.push(format!("{}\t{}\n", dir, name));
        }
    }
    lines.sort();
    use std::io::Write;
    let written = std::fs::OpenOptions::new().create(true).append(true).open(path).and_then(|mut f| f.write_all(lines.concat().as_bytes()));
    if let Err(e) = written {
        eprintln!("cannot write {}: {}", path, e);
    }
}

/// A check build's `inits`: the objects of the owned sources whose construction runs code, by
/// the rule the JVM build applies to the module classes it emits (`products::object_inits`).
fn check_inits(typer: &typer::Worker, entries: &mut [products::Entry]) {
    let layout = emit::layout::Layout::new(&typer.prog, &typer.syms, typer.interner);
    let owned = |f: source::FileId| (f.0 as usize) < typer.files.len() && typer.is_program_file(f) && !typer.in_jar(f);
    let objects: Vec<types::ClassId> = typer.prog.classes.iter().map(|tc| tc.id).filter(|&c| owned(typer.syms.class(c).file) && typer.syms.class(c).owner != symbols::Owner::Local).collect();
    for c in objects {
        let Some(name) = products::object_inits(&typer.syms, typer.interner, &layout, std::iter::once(c)).pop() else { continue };
        let source = &typer.files.as_slice()[typer.syms.class(c).file.0 as usize].path;
        // The object's entry: its source's product whose class holds it, as the JVM's class
        // files fall under their products (`mark_products`).
        let best = entries
            .iter_mut()
            .filter(|e| &e.source == source)
            .filter(|e| e.tasty.as_deref().and_then(|t| t.strip_suffix(".tasty")).map_or(false, |stem| name == stem || name.starts_with(&format!("{}$", stem))))
            .max_by_key(|e| e.tasty.as_ref().map_or(0, String::len));
        if let Some(e) = best {
            e.inits.push(name);
        }
    }
}

/// A build's own products: the entries it made with the bytes of their files, and the std classes
/// it wrote, before they meet the directory's.
#[derive(Default)]
struct Fresh {
    entries: Vec<products::Entry>,
    files: Vec<(String, Vec<u8>)>,
    std: Vec<(String, Vec<u8>)>,
}

/// What a build publishes into its directory of products, and the answer's `generated` journal:
/// in a build against the module's own directory (`own`), the entries it retains with the
/// build's, the dropped entries' files and the std classes no entry names any more removed;
/// otherwise the build's alone, as the directory's whole products (docs/TARGETS.md, "The contract
/// between the plugin and the compiler").
fn products_publication(own: Option<&classpath::OwnRead>, target: &str, mut fresh: Fresh) -> Result<(products::Publication, String), String> {
    use crate::lsp::json::Json;
    let root = std::env::current_dir().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
    let mut owner: intern::FxMap<String, String> = intern::FxMap::default();
    {
        let bytes: intern::FxMap<&str, &[u8]> = fresh.files.iter().map(|(p, b)| (p.as_str(), b.as_slice())).collect();
        for e in fresh.entries.iter_mut() {
            e.digest = products::digest(e.files().map(|f| (f, bytes.get(f).copied().unwrap_or_default())));
            for f in e.files() {
                if let Some(other) = owner.insert(f.to_string(), e.source.clone()).filter(|o| *o != e.source) {
                    return Err(format!("internal error: {} is a product of both {} and {}", f, other, e.source));
                }
            }
        }
    }
    products::sort_entries(&mut fresh.entries);
    let mut entries: Vec<products::Entry> = Vec::new();
    let mut dropped: Vec<&str> = Vec::new();
    let old = own.and_then(|o| o.manifest.as_ref().map(|m| (m, &o.retained)));
    if let Some((m, retained)) = old {
        for (e, &keep) in m.entries.iter().zip(retained.iter()) {
            if !keep {
                dropped.extend(e.files());
                continue;
            }
            // A file the build writes again for another source (a definition moved, or made
            // twice) is the build's; the entry keeps listing it, so that the next compile, which
            // zinc makes of both sources, settles the two.
            let mut e = e.clone();
            // An entry of a manifest written under another root names its source as
            // `classpath::entry_source` reads it, this checkout's.
            if m.root != root {
                let path = classpath::entry_source(m, &e.source);
                e.source = if std::path::Path::new(&e.source).is_absolute() {
                    path.to_string_lossy().to_string()
                } else {
                    let base = classpath::canonical_lenient(std::path::Path::new(&root));
                    path.strip_prefix(&base).unwrap_or(&path).to_string_lossy().to_string()
                };
            }
            entries.push(e);
        }
    }
    let generated_rows = generated_journal(&fresh.entries);
    entries.extend(fresh.entries);
    let mut manifest = products::Manifest { target: target.to_string(), root, entries, std: Vec::new(), legacy_inits: Vec::new() };
    manifest.sort();
    // A dropped entry's file another entry lists stays: the build's, or a retained entry's that
    // took it over in an earlier build.
    let listed: intern::FxMap<&str, ()> = manifest.entries.iter().flat_map(|e| e.files()).map(|f| (f, ())).collect();
    let mut removed: Vec<String> = dropped.into_iter().filter(|f| !listed.contains_key(f)).map(str::to_string).collect();
    let mut std: Vec<String> = manifest.entries.iter().flat_map(|e| e.std.iter().cloned()).collect();
    std.sort();
    std.dedup();
    if let Some((m, _)) = old {
        removed.extend(m.std.iter().filter(|s| std.binary_search(s).is_err()).cloned());
    }
    let written_std: Vec<(String, Vec<u8>)> = fresh.std.into_iter().filter(|(p, _)| std.binary_search(p).is_ok()).collect();
    let mut rows = generated_rows;
    rows.push(Json::Obj(vec![("source".to_string(), Json::Null), ("files".to_string(), Json::Arr(written_std.iter().map(|(p, _)| Json::Str(p.clone())).collect()))]));
    manifest.std = std;
    let mut files = fresh.files;
    files.extend(written_std);
    Ok((products::Publication { files, removed, manifest: manifest.to_json() }, Json::Arr(rows).to_text()))
}

/// The answer's `generated` rows of a build's own entries, in the manifest's order: per source,
/// every file of its products.
fn generated_journal(entries: &[products::Entry]) -> Vec<crate::lsp::json::Json> {
    use crate::lsp::json::Json;
    let mut rows: Vec<(String, Vec<Json>)> = Vec::new();
    for e in entries {
        if rows.last().map_or(true, |(s, _)| *s != e.source) {
            rows.push((e.source.clone(), Vec::new()));
        }
        rows.last_mut().unwrap().1.extend(e.files().map(|f| Json::Str(f.to_string())));
    }
    rows.into_iter().map(|(s, files)| Json::Obj(vec![("source".to_string(), Json::Str(s)), ("files".to_string(), Json::Arr(files))])).collect()
}

/// Publishes a build's products, then prints its answer; a build that cannot publish answers
/// that it failed.
fn publish_products(opts: &Options, dir: &str, publication: products::Publication, answer: Option<String>) {
    if let Err(e) = publication.publish(std::path::Path::new(dir)) {
        eprintln!("cannot write {}: {}", dir, e);
        exit_failed(opts, 2);
    }
    print_answer(answer);
}

/// `--removed` without sources: the module's manifest without the removed sources' entries, their
/// files and the std classes no entry names any more removed, published as a build's products
/// are; a directory without a manifest is left as it is.
fn manifest_only(opts: &Options) -> ! {
    let dir = opts.products.as_deref().unwrap_or_default();
    let manifest = match products::Manifest::read(std::path::Path::new(dir)) {
        None => None,
        Some(Ok(m)) => Some(m),
        Some(Err(e)) => {
            eprintln!("{}", e);
            exit_failed(opts, 2);
        }
    };
    let answer = opts.analysis_version.map(|v| format!("{{\"ok\":true,\"analysisVersion\":{},\"analysis\":[],\"api\":{{\"nodes\":[],\"files\":[]}},\"generated\":[]}}\n", v));
    let Some(m) = manifest else {
        print_answer(answer);
        leave(0)
    };
    let dropped: Vec<std::path::PathBuf> = opts.removed.iter().map(|p| classpath::canonical_lenient(std::path::Path::new(p))).collect();
    let retained: Vec<bool> = m.entries.iter().map(|e| !dropped.contains(&classpath::entry_source(&m, &e.source))).collect();
    let target = m.target.clone();
    let own = classpath::OwnRead { entry: 0, manifest: Some(m), retained };
    match products_publication(Some(&own), &target, Fresh::default()) {
        Ok((publication, _)) => publish_products(opts, dir, publication, answer),
        Err(e) => {
            eprintln!("{}", e);
            exit_failed(opts, 2);
        }
    }
    leave(0)
}

fn print_answer(answer: Option<String>) {
    if let Some(line) = answer {
        use std::io::Write as _;
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(line.as_bytes());
        let _ = out.flush();
    }
}

/// `teq interp`: the program runs in the interpreter over the typed IR, typing the bodies of
/// the standard library as it reaches them.
fn run_interp(opts: &Options, mut typer: typer::Worker, lines: usize, t: [Instant; 4]) -> i32 {
    let mut interp = interp::Interp::new(&mut typer, interp::Limits::unlimited());
    interp.stream = true;
    interp.content_hashes = false;
    let result = interp.run_main(&opts.program_args);
    interp.flush();
    if std::env::var_os("TEQ_INTERP_TRACE").is_some() {
        eprintln!("interp: {} steps", u64::MAX - interp.steps_left());
    }
    let t_run = Instant::now();
    let mut code = match result {
        Ok(()) => 0,
        // A program's own status passes through, as the JVM's process exits with it.
        Err(interp::Failure::Exit(status)) => status,
        Err(f) => {
            let message = interp.describe(&f);
            eprintln!("{}", message);
            match f {
                interp::Failure::Thrown(_) | interp::Failure::Withheld => 1,
                _ => 3,
            }
        }
    };
    // The profile after the run, which typed the std's bodies the program reached.
    if opts.profile {
        print_profile(&mut typer, t[3] - t[2], opts.profile_json.as_deref());
    }
    if !typer.diags.items.is_empty() {
        eprint!("{}", typer.render_diags());
    }
    // An error the run met (a jar entry a lookup could not read) fails the command.
    if code == 0 && typer.diags.has_errors() {
        code = 1;
    }
    save_jar_caches(&mut typer);
    if opts.timings {
        let mut report = report::Report::default();
        report.phases(lines, &[("read", t[1] - t[0], ""), ("parse", t[2] - t[1], ""), ("type", t[3] - t[2], ""), ("run", t_run - t[3], "")], t_run - t[0]);
        eprintln!("{}", report.render());
    }
    code
}

#[cfg(test)]
mod identity_tests {
    use super::shortest_unique_suffixes;

    fn parts(paths: &[&str]) -> Vec<Vec<String>> {
        paths.iter().map(|p| p.split('/').filter(|c| !c.is_empty()).map(str::to_string).collect()).collect()
    }

    #[test]
    fn a_file_alone_goes_by_its_name_and_beside_a_namesake_by_one_component_more() {
        assert_eq!(shortest_unique_suffixes(&parts(&["/work/b/Use.scala"]), vec![1]), vec!["Use.scala"]);
        let two = parts(&["/work/a/Use.scala", "/work/b/Use.scala"]);
        assert_eq!(shortest_unique_suffixes(&two, vec![1, 1]), vec!["a/Use.scala", "b/Use.scala"]);
    }

    #[test]
    fn the_root_tells_two_paths_apart_whatever_their_order() {
        let given = parts(&["/src/Use.scala", "/work/src/Use.scala"]);
        assert_eq!(shortest_unique_suffixes(&given, vec![1, 1]), vec!["src/Use.scala", "work/src/Use.scala"]);
        let reversed = parts(&["/work/src/Use.scala", "/src/Use.scala"]);
        assert_eq!(shortest_unique_suffixes(&reversed, vec![1, 1]), vec!["work/src/Use.scala", "src/Use.scala"]);
    }

    #[test]
    fn a_file_under_a_root_keeps_its_path_under_the_root() {
        let files = parts(&["/x/shared/src/io/Foo.scala", "/x/app/src/io/Foo.scala"]);
        assert_eq!(shortest_unique_suffixes(&files, vec![2, 2]), vec!["shared/src/io/Foo.scala", "app/src/io/Foo.scala"]);
    }
}

/// The program `text` typed as `teq compiler check` types it with `flags`, for the unit tests of the
/// typer's parts that read a typed program: `f` runs on its worker once the bodies are typed.
/// The source is written to a directory of its own under the system's temporary directory.
#[cfg(test)]
pub(crate) fn with_typed_program<R: Send>(name: &str, text: &str, flags: &[&str], f: impl FnOnce(&mut typer::Worker) -> R + Send) -> R {
    // The typing thread's exit hands its caches back to the allocator's centre, whose counts
    // the allocator's tests read.
    let _serial = alloc::serial_test();
    let dir = std::env::temp_dir().join(format!("teq-unit-{}-{}", std::process::id(), name));
    std::fs::create_dir_all(&dir).expect("a directory for the test's source");
    let path = dir.join(format!("{}.scala", name));
    std::fs::write(&path, text).expect("the test's source written");
    let mut args = vec!["compiler".to_string(), "check".to_string(), path.to_string_lossy().into_owned()];
    args.extend(flags.iter().map(|f| f.to_string()));
    std::thread::scope(|scope| {
        let typed = std::thread::Builder::new().stack_size(1 << 30).spawn_scoped(scope, move || {
            let opts = parse_options_from(args.into_iter());
            let paths = collect_program_paths(&opts);
            let keys = program_keys(&opts, paths.iter().map(String::as_str));
            let program: Vec<SourceFile> = paths.into_iter().zip(keys).map(|(path, key)| SourceFile { text: std::fs::read_to_string(&path).expect("the test's source"), path, is_std: false, key }).collect();
            let mut interner = intern::Interner::new();
            let mut diags = Diagnostics::default();
            let cp = open_jars(&opts);
            let (mut files, mut asts, blocks_of, std, selected, scalajs_unlocked) = parse_build(&opts, cp.as_ref(), program, &mut interner, &mut diags);
            assert!(!diags.has_errors(), "the test's source does not parse");
            let (jar_files, jdk_file) = jar_pseudo_files(&opts, &mut files, &mut asts);
            let asts = Asts::new(asts);
            let files = Sources::new(files);
            let mut typer = typer::Typer::new(&asts, &files, &mut interner);
            typer.threads = 1;
            install_std(&mut typer, std, &selected, &blocks_of, scalajs_unlocked);
            open_classpath(&opts, &mut typer, cp, jar_files, jdk_file);
            typer.dialect = opts.dialect;
            typer.inline.max_depth = opts.max_inlines;
            typer.inline.outlines = !opts.jvm && !opts.interp && !opts.no_outline;
            if opts.jvm {
                typer.set_jvm();
            }
            typer.run();
            let mut f = Some(f);
            let mut out = None;
            typer.on_thread(|w| out = Some((f.take().expect("run once"))(w)));
            out.expect("the worker ran the test")
        });
        typed.expect("the test's typer thread").join().expect("the test's typer panicked")
    })
}
