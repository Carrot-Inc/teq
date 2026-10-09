# Changelog

What each release of teq changes for a user: the compiler, the build tool and the sbt plugin, the editor
support, and the platforms the binaries run on. The newest release comes first; its section is also the
notes of its GitHub release, which carries the binaries (`teq-<version>-<classifier>`, the Windows one with `.exe`) beside their
`SHA256SUMS`.

## 0.1.7 (2026-10-08)

- The binaries are published as GitHub releases: `v0.1.7` carries `teq-0.1.7-<classifier>` for each
  platform (the Windows one with `.exe`) beside a `SHA256SUMS` and a manifest of their digests and sizes,
  `teq-0.1.7-binaries.txt`. The sbt plugin is published to Maven Central as `build.teq:sbt-teq` under a version
  line of its own, `1.0.0` first, which serves this compiler and the releases after it; a build names the compiler
  by `teqVersion` (the plugin's own default is the compiler it was released with) and the plugin fetches the
  binary from the release, checked against the manifest before it runs.
- The releases before 0.1.7 are no longer served by the plugin, the launchers or the Zed extension: a build naming
  one, or a lock pinning one, is refused ("releases before 0.1.7 are not served: pin 0.1.7 or later") and moves to
  0.1.7 with the plugin 1.0.0. The launchers an export writes refuse such a lock, and a lock without the platform's
  binary, naming `TEQ` and the release to pin; a build's committed launchers are replaced at its next export.
- The binaries for macOS on Intel and Linux on ARM64, held back from 0.1.6 until they had run on their
  platforms.
- A build's source generator can be a Scala script run by teq's own interpreter: a `TeqCommand` whose first
  word is `teq` runs the binary the build runs with (sbt's resolved one, or the one running `teq compile`),
  never a `teq` found on the PATH, so `TeqCommand(Seq("teq", "interp", "scripts/gen.scala", "--", "app"))`
  needs no JVM, scala-cli or shell on any machine. A program under `teq interp` has the JDK's files
  (`Files`' readers and writers, `createDirectories`, `list` and `walk` as streams, `Path`'s `relativize`,
  `normalize` and `toAbsolutePath`, the JDK's exceptions), its process's working directory and environment,
  and `sys.exit`'s status. `TeqCommand` gains `outputs`, the files a generator writes besides its sources,
  which a missing one makes it run again. A macro still writes no file.
- The sbt setting `teqDriver` is now `teqBuildTool`. A build that set `teqDriver := true` sets
  `teqBuildTool := true` when it moves to this release, then runs `sbt teqExportAll` and commits the new
  lock; the old name no longer makes teq the build tool.
- A build file that differs from the lock's record by its line ends alone, as on a CRLF checkout (Git for
  Windows' default), is named so where the lock is reported stale (`teq`'s verbs, the plugin and the
  language server), with the remedy: `*.sbt text eol=lf` and the build's other inputs in `.gitattributes`,
  or an export on that machine. The lock is still stale, and `--strict` still refuses it.
- Trait parameters compile on the JVM, laid out as scalac lays them out: the class that mixes the trait in
  holds a field per parameter and sets it before the trait's initialiser, a left-out argument coming from
  the companion's default. The accessors of a trait's private and using parameters take scalac's names
  (`T$$x`), which changes the class files' ABI from 0.1.6's.
- On the JVM as well: a Scala 2.13 trait's private vars, lazy vals and nested objects are read from its
  class file; a `@volatile` field is written volatile; a generic trait's members implemented by a
  superclass's givens get their bridges; a product trait's given definitions stay in its own module.
- Abstract and deferred givens, as Scala 3.8 has them: `given x: T` with no body is an abstract given, and
  a trait's `given x: T = deferred` is implemented in each concrete class below the trait by a search in
  the class's context, with scalac's errors. scalac reads both from teq's output, and teq from scalac's.
- Extension methods resolve as Scala 3 resolves them: an extension method is bound by name where an application's
  head names it (an object's, a class's or an imported value's), a selection takes the nearest scope's extensions
  (a named import over its scope's wildcards, an inner import against an outer definition ambiguous), and
  `"x".+:(2)` with a right-associative extension means `2 +: "x"`, the qualifier evaluated first. A minus before a
  number across spaces is the number's sign; a suffixed number out of range is an error at the number; a method in
  statement position that would eta-expand is an error (E178); a refutable pattern's failing part is reported
  against its own scrutinee type; a missing given is reported at the end of what the call applies; a written
  context function literal stays a function where an ascribed one is applied; the mutable map's `map`, `collect`
  and `flatMap` are overloaded as scala-library's.
- Given classes and objects of a class or trait instance capture their enclosing instance on every target, whole
  and split, with scalac's JVM layout (`Owner$x` with its outer, the owner's factory, a trait's default method); an
  enum instance's as well. A product body passing a using parameter to an ordinary clause is read by the clause
  the call fills; a generic trait parent's type arguments are inferred from its arguments; a library body's
  expansions survive a retracted typing attempt, and a withheld body names its producer, reason and call site; a
  case class's synthesized `Mirror` is written as scalac's companion cast, so a derived type class runs
  downstream; a plain private constructor parameter is scalac's `private final` field with no accessor; a case
  class's companion is initialised before its construction's arguments run, in the interpreter and in a module
  built over its products, as scalac's `apply` has it.
- Every release carries `NOTICE` and `LICENSE` beside its binaries, and `NOTICE` credits each adapted file in the
  form its license requires (the Zed extension's language files from metals-zed, the hot-reload runtime from
  react-refresh, the DOM facades from scalajs-dom, the number formatting from scala-java-locales, the character
  tables from the Unicode Character Database) and says which notices a compiled program carries; the vite package
  and the Zed bundle ship the same two files.
- A JDK's `lib/ct.sym` is read at the newest release it holds class files for: OpenJDK 21's archive has a directory
  for release 21 with its module list alone, which made teq read no JDK class at all under that JDK (`java.util`'s
  members "not a member" at a macro reading a resource bundle).
- `java.util.stream.Stream` has the JDK's members on JavaScript and in the interpreter: `map`, `filter`, `flatMap`,
  `sorted`, `distinct`, `limit`, `skip`, `peek`, `collect` with `Collectors` (`toList`, `toSet`, `toMap`, `joining`,
  `counting`, `groupingBy`), `reduce`, `count`, the matches and finds, `iterate`, `generate`, `concat`, the primitive
  streams' terminal operations, `Optional` and its primitive versions, as a lazy single-use pipeline with the JDK's
  short-circuiting, close handlers and exceptions; the JVM types `Stream` against the JDK's own class. A sequence or
  array spread into a Java varargs reaches the method as it does under scalac: a plain array of references as itself,
  a written conversion copied; an interface's static method is called correctly on the JVM.
- `println(())` and `print(())` print `undefined` on JavaScript and in the interpreter, as under Scala.js, and `()`
  on the JVM; `println` has scala-library's two overloads and a bare `println` prints an empty line; on the JVM a
  Scala 2 library's `def f()` is called on a bare reference as scalac calls it.
- Extension methods in lexical scope are selected by Scala 3's rule: a literal argument typed against each
  candidate's own parameter, the receiver and the leading using clauses resolved before the method's own
  parameters, imported candidates tried one at a time (two that apply are ambiguous), and the implicit
  scope's extensions ranked as givens are.
- A lambda that matches on its parameter, passed where a `PartialFunction` is expected, is the partial
  function, through an overload too: `map.collect(kv => kv._2 match { ... })` no longer warns that the
  match may not be exhaustive (an error under `-Werror`), and between `Function1` and `PartialFunction`
  overloads the partial function is chosen, as scalac chooses it. E211 is reported at a match that ends a
  partial function's block.
- Pattern matching follows scalac more closely: an impossible type test is error E030; a constructor
  pattern after an irrefutable extractor is checked by the case's kind; a binder's type is `B & A`, in
  scalac's order; an invariant type parameter left open is bound to a fresh abstract type, so
  `val y: Box[Any] = b` is rejected as scalac rejects it; an abstract `unapply`'s input is checked with
  E092.
- Navigation in the editor (definition, references, hover, the call hierarchy) finds what the Scala 3
  compiler's interactive API finds for more expressions: a closure called through its written `apply`, a
  transparent inline method's lambda applied at once, a by-name parameter, `arr(0)` and `str(0)`, a
  class-backed given's member, an object's inline `apply`, a `for` binder at its pattern, an ascribed
  receiver such as `(obj: Base).f`, and a call through an implicit conversion.
- The language server no longer shows its idle poll as a build: on Windows, where a poll is slow, the
  status bar said `checking <project>` several times a minute. The idle period runs from the last answer,
  and a build whose generators fail no longer runs them again at every wake.
- sbt: a compile restored from sbt's cache by another checkout of the same sources keeps its products (the
  products' record is read relative to the checkout), and a cached link whose target is gone no longer
  breaks every later compile until `clean`. `docs/TOOLING.md` says why `test` after `clean` can run no
  suite: sbt 2's `test` is `testQuick`, whose successes stay in sbt's disk cache.
- Numbers and literals as scalac reads them: an unsuffixed decimal where a `Float` is expected is typed from its
  digits (`val f: Float = 1.5` compiles; `1e40` is "number too large"); a `Double` elsewhere, as before.
- Overloads and applications: a named function literal to an overloaded method is refused where scalac refuses it;
  a `for` over a `Map` with a tuple pattern picks the right `map`; a context function an inferred definition yields
  is applied to the givens in scope (and refused without one, as under scalac); `ops(2)` on a value named `apply`
  is refused (E050); a statement begun on a brace's line ends at that line's break.
- Right-associative extension methods called as methods take their parameter lists in the declaration's order and
  evaluate their arguments as written; an import through a stable `val` of the enclosing instance works.
- A class nested in a local or anonymous class (a member, a nested object, a case class and its companion) reads
  the enclosing method's locals on every target; a JVM lazy val whose value is `null` is computed once;
  `x.equals(y)` on a receiver typed `Any`, a primitive or a type parameter calls the method, as Scala.js and the
  JVM do; a class mixing in a library trait that overrides `toString`, `hashCode` or `equals` gets the trait's; a
  package-level `main` is found by the run commands and an object's `@main` method can be launched by its name.
- Products a downstream module compiles against: the std's `String.toLowerCase`, `toUpperCase`, `contains` and
  `replace` are written as the JDK's, so an inline interpolator of a main module expands in a module built over its
  products; an export forwarder's pickle keeps its `@targetName`; an anonymous using parameter is named as scalac
  names it; a trait's `abstract override` over a generic parent has its super accessor typed as the trait sees it;
  a product's given class is read back as the program's own.
- The parser's recovery of the mutant corpus is the compiler's rule for rule: two cascades the typer added after an
  overload error are gone (`No ClassTag available for Nothing`).

## 0.1.6 and before

- teq compiles Scala 3 to JavaScript and to JVM class files and runs programs in its interpreter, checked
  against scalac 3.8.4 and the Scala compiler's own suites.
- The sbt plugin puts teq in the place of sbt's compiler (`teqCompiler := true`) for JVM
  and Scala.js projects, and exports the build into `teq.lock`, from which `teq`'s project verbs (compile,
  test, run, stage, watch, build, dev), the launchers, the vite plugin and the language server work with no
  sbt running; the Zed extension runs the language server.
- The binaries: macOS on Apple silicon from 0.1.0-pre.1, Linux on x86-64, and Windows on
  x86-64 from 0.1.6. 0.1.6's Linux binary loads on glibc 2.28 and later, the first to run on Ubuntu 22.04
  and Debian 11.
- 0.1.6 brought argument files for every command with a class path (Windows' limit on a command line), the
  project verbs, an export that never refuses, the opt-in build tool (`teqDriver`), the stage's implied
  main class, a jar trait's fields, and the export's binaries without downloads.
