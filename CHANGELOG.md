# Changelog

What each release of teq changes for a user: the compiler, the build tool and the sbt plugin, the editor
support, and the platforms the binaries run on. The newest release comes first; its section is also the
notes of its GitHub release, which carries the binaries (`teq-<version>-<classifier>`, the Windows one with `.exe`) beside their
`SHA256SUMS`.

## 0.1.8 (2026-10-09)

- A build file git ignores, such as Metals' `project/metals.sbt`, no longer makes `teq.lock` stale: the warning
  naming `sbt teqExportAll` comes only for the build's own files.
- A command generator runs again only when its inputs changed or an output its last run made is gone; one whose
  managed-sources directory stays empty, a script writing its files elsewhere, no longer runs on every check of
  `teq dev` ([generators](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md)).
- Every teq process raises its open-files limit at start, so a build whose class path holds hundreds of jars runs
  from a macOS shell's default of 256 descriptors.
- `teq interp` runs programs with the JDK's processes, pipes and signals, sockets, files and directories, digests,
  zip archives and date-time formatting, and takes a shebang line and `//> using file`; a repository can run its own
  scripts as Scala programs through its `teq.lock` ([`teq interp`](https://github.com/Carrot-Inc/teq/blob/master/docs/CLI.md)).
- vite-plugin-teq reads `teq.lock` through the `yaml` package; an install from a checkout with npm runs
  `npm install --legacy-peer-deps` once in the plugin's directory ([the vite plugin](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md#the-vite-plugin)).
- The dev server's hot swap survives a local and a non-local binding of one name, and a library module's output
  no longer changes with the order a program reads its values.
- Products a downstream module compiles against: a refinement method's parameter references, a polymorphic
  function type's signature and a by-name parameter are written as scalac writes them, so scalac reads teq's
  products and teq scalac's for those shapes; a block's classes and a withheld library body's diagnostic are the
  same at every worker count ([products](https://github.com/Carrot-Inc/teq/blob/master/docs/TARGETS.md)).
- `asInstanceOf` is checked on JavaScript and in the interpreter, and on the JVM before the value is used, as
  scalac's erasure checks it; `null` goes through a cast to a reference type and unboxes to a primitive's zero. A cast to a function type takes a function of any arity outside the JVM, where
  Scala.js's `js.FunctionN` are scala's function types and a JavaScript function's `length` is not its arity
  ([conformance](https://github.com/Carrot-Inc/teq/blob/master/docs/COMPATIBILITY.md)).

<details>
<summary>Conformance with scalac and the JDK</summary>

- An explicit `toString` call keeps its value, `null` included, and a value is rendered where the library
  renders it, as `String.valueOf` and `addString` do.
- `Enum.ordinal` dispatches to the receiver's implementation before the case's, with `null`'s ordinary failure,
  and a jar enum's cases are numbered as its pickle encodes them.
- A Java `final` field with a constant value is typed by its constant and folded, `Math.PI` among them.
- A given with type parameters is a def; a concrete `var`'s setter is a member; an inline accessor is named as
  scalac names it, in the pickle and the class file.
- A macro's source path is the path the build was given; a quote pattern's type variables are solved under one
  constraint within their declared bounds.
- A package's members include its package object's inherited ones over the products too; an anonymous class's
  definition sits at its `new`; a synthesized mirror leaves out the members its companion declares; a
  `Char`-bounded type parameter erases as scalac erases it.
- A cast is decided after inline substitution on the erased types: a redundant one is an ascription, a cast to
  `Unit` evaluates its operand, a primitive converts, a value class is tested as its box outside the JVM, a cast
  to `Nothing` throws; a cast a type test on the same local proved is free.
- Case classes, tuples and enum cases are `Product`, `Equals` and `Serializable` at run time on every target; a
  string or number is `Comparable` and `CharSequence` on JavaScript as under Scala.js.
- The JDK's collections on JavaScript and in the interpreter: removal through an iterator and `removeIf`, the map
  and set equalities, `IdentityHashMap`, the primitive streams, `addSuppressed`, `java.util.Random` and
  `BigInteger`'s random and prime constructors, the boxes' constructors, `Float` from a string as the JDK parses it.
- Regular expressions: the POSIX classes are ASCII, atomic groups and `(?U)` work in the interpreter, a bad pattern
  fails at `Pattern.compile`.
- Structural calls through `reflectiveSelectable` carry their class arguments; a sequence spread into Java varargs
  reaches the method; `new js.Array[A]()`, `sort()` and `reverseInPlace()` are Scala.js's.
- A refinement member is checked as scalac checks it: `PolyFunction` refinements with a method member alone, no
  overload and no by-name `apply`; a parameter's mode is part of a refinement's conformance.
- A tuple `Ordering` exists to arity nine; `Range` reaches `Int.MaxValue`; `Date.toInstant` on the JVM; the
  Scala 2 bare calls with scalac's error.
- A local `$$X` beside a non-local `$$X` and an all-dollar object's accessor beside an all-dollar local run on
  JavaScript as named.
- `TimeZone.setDefault` reaches `java.time` on the JVM; the interpreter's archive times take the machine's zone
  ([compatibility](https://github.com/Carrot-Inc/teq/blob/master/docs/COMPATIBILITY.md)).

</details>

## 0.1.7 (2026-10-08)

- Binaries are published as GitHub releases, five platforms beside `SHA256SUMS` and a manifest; the sbt plugin is on
  Maven Central as `build.teq:sbt-teq` 1.0.0 and fetches the compiler `teqVersion` names ([the sbt plugin](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md#the-sbt-plugin)).
- Releases before 0.1.7 are no longer served: a build or a lock naming one is refused with the version to pin.
- Binaries for macOS on Intel and Linux on ARM64, held back from 0.1.6 until they had run on their platforms.
- A build's generator can be a Scala script run by teq's interpreter: a `TeqCommand` starting with `teq` runs the
  build's own binary, and a `teq interp` program has the JDK's files, its working directory, environment and `sys.exit`
  ([generators](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md#the-sbt-plugin), [`teq interp`](https://github.com/Carrot-Inc/teq/blob/master/docs/CLI.md)).
- The sbt setting `teqDriver` is now `teqBuildTool`; a build that renames it exports again and commits the lock.
- A lock stale by line ends alone (a CRLF checkout) is reported as such, with the `.gitattributes` remedy
  ([the lockfile](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md#the-lockfile-and-the-launchers)).
- Editor navigation (definition, references, hover, call hierarchy) resolves more expressions: a closure called
  through `apply`, a by-name parameter, `arr(0)`, a `for` binder, an ascribed receiver, an implicit conversion.
- The language server's idle poll no longer shows as a build on Windows, and failing generators are not rerun at
  every wake.
- sbt: a compile restored from the cache by another checkout keeps its products, and a cached link whose target is
  gone no longer breaks later compiles.
- A JDK's `lib/ct.sym` is read at its newest release with class files: under OpenJDK 21 no JDK class was read at all.
- `NOTICE` and `LICENSE` ship with every release, the vite package and the Zed bundle, crediting each adapted file.

<details>
<summary>Conformance with scalac and the JDK</summary>

- JVM: trait parameters are laid out as scalac's, with `T$$x` accessors; the class files' ABI changes from 0.1.6's.
- JVM: a Scala 2.13 trait's private vars, lazy vals and nested objects are read from its class file; `@volatile`
  fields are written volatile; members implemented by a superclass's givens get their bridges.
- Abstract givens (`given x: T` without a body) and deferred givens (`= deferred`), interoperable with scalac's output.
- Extension methods resolve by Scala 3's rule: bound by name at an application's head, the nearest scope's extensions
  first, imported candidates one at a time, the implicit scope's extensions ranked as givens.
- `"x".+:(2)` with a right-associative extension means `2 +: "x"`; called as a method, the parameter lists keep the
  declaration's order and the arguments evaluate as written.
- A minus before a number across spaces is its sign; an out-of-range suffixed number is an error at the number; an
  unsuffixed decimal typed `Float` is read from its digits.
- A method in statement position that would eta-expand is an error (E178); `ops(2)` on a value named `apply` is
  refused (E050); a named function literal to an overloaded method is refused as scalac refuses it.
- A refutable pattern's failing part is reported against its own scrutinee; a missing given is reported at the end of
  the call; E211 is reported at a match that ends a partial function's block.
- A written context function literal stays a function where an ascribed one is applied; an inferred definition that
  yields a context function is applied to the givens in scope.
- The mutable map's `map`, `collect` and `flatMap` are overloaded as scala-library's; a `for` over a `Map` with a tuple
  pattern picks the right `map`.
- Given classes and objects of an instance capture their enclosing instance on every target, with scalac's JVM
  layout; an enum instance's too.
- A product body's using argument to an ordinary clause is read by the clause the call fills; a generic trait parent's
  type arguments are inferred from its arguments.
- A case class's synthesized `Mirror` is written as scalac's companion cast, so a derived type class runs downstream;
  its companion is initialised before its construction's arguments run.
- A plain private constructor parameter is scalac's `private final` field with no accessor.
- A library body's expansions survive a retracted typing attempt; a withheld body names its producer, reason and call
  site.
- `java.util.stream.Stream` has the JDK's members on JavaScript and in the interpreter, a lazy single-use pipeline with
  `Collectors` and `Optional`; on the JVM it is the JDK's class.
- A sequence or array spread into a Java varargs reaches the method as under scalac; an interface's static method is
  called correctly on the JVM.
- `println(())` prints `undefined` on JavaScript and in the interpreter and `()` on the JVM; `println` has
  scala-library's two overloads; a Scala 2 library's `def f()` is called on a bare reference.
- A lambda matching on its parameter where a `PartialFunction` is expected is the partial function, through
  overloads too, with no exhaustivity warning.
- Pattern matching: an impossible type test is E030; a constructor pattern after an irrefutable extractor is checked
  by the case's kind; a binder's type is `B & A`; an open invariant type parameter is a fresh abstract type; an
  abstract `unapply`'s input is checked (E092).
- A statement begun on a brace's line ends at that line's break; an import through a stable `val` of the enclosing
  instance works.
- A class nested in a local or anonymous class reads the enclosing method's locals on every target; a JVM lazy val
  whose value is `null` is computed once.
- `x.equals(y)` on `Any`, a primitive or a type parameter calls the method; a class mixing in a library trait that
  overrides `toString`, `hashCode` or `equals` gets the trait's.
- A package-level `main` is found by the run commands; an object's `@main` method launches by its name.
- Products a downstream module compiles against: the std's `String.toLowerCase`, `toUpperCase`, `contains` and
  `replace` are the JDK's; an export forwarder keeps its `@targetName`; anonymous using parameters and super
  accessors are named and typed as scalac's; a product's given class is read back as the program's own.
- The parser's recovery follows the compiler's rule for rule; two error cascades after an overload error are gone.

</details>

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
