# Changelog

What each release of teq changes for a user: the compiler, the build tool and the sbt plugin, the editor
support, and the platforms the binaries run on. The newest release comes first; its section is also the
notes of its GitHub release, which carries the binaries (`teq-<version>-<classifier>`, the Windows one with `.exe`) beside their
`SHA256SUMS`.

## Unreleased

- The editor offers one action that adds every missing import of a file at once, "Import all missing names"
  (`source.addMissingImports`): the import of each name that has exactly one, written as the single quick fixes
  write them, several names of one prefix into its clause in one edit; a name with two candidates keeps its own
  quick fixes
  ([the language server](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md#the-language-server-and-zed)).
- An inline method's expansion reads a cast by the cast's own type, as scalac does: `inline if n.asInstanceOf[Int] > 5`,
  an `inline match` over such a cast and a by-value argument under one are no constant (a correct program's other
  case is taken; an `inline if` that scalac refuses is refused), where a cast to a singleton type (`n.asInstanceOf[9]`,
  or to the type of a constant val, `n.asInstanceOf[Limits.max.type]`) is that constant whatever stands under it. The
  condition under such a cast is still evaluated before the branch it selects.

## 0.1.10 (2026-11-13)

- `teq fix` runs scalafix's rules over a build's sources with no sbt and no JVM: the rules it ships and a
  project's own (the configuration's `file:` entries) over scalameta 4.17's tree, the configuration's `include`s
  and substitutions, `--rules` and `--check`; `OptionGetWarn`, `ForbiddenSymbol` and
  `TypelevelHttp4sLiteralsSyntax` are answered by the compiler's own typing, which gives a semantic rule the
  names scalac's SemanticDB has, those of an annotation's arguments and of a method an inline method calls
  among them
  ([`teq fix`](https://github.com/Carrot-Inc/teq/blob/master/docs/FIX.md)).
- sbt-teq 1.0.2 records in `teq.lock` the configuration file sbt-scalafix reads for each configuration, and the
  ordered groups of `teqTestGroups` ([the lockfile](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md)).
- The editor offers quick fixes that add a missing import: for a name that is not found, the imports completion
  offers and the objects of the program that hold a member of that name; for a method a value lacks
  (`42.asGlyph`), the objects whose extension method the selection then finds. An import the editor adds,
  completion's and a quick fix's, goes into the clause a file already has of that prefix (`import a.b.X` becomes
  `import a.b.{X, Z}`, the name at its sorted place where the clause is in order)
  ([the language server](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md#the-language-server-and-zed)).
- `teq test` runs a configuration's suites in ordered groups, lists a run with `--list`, and loads and calls the
  test frameworks as sbt 2.0.8's forked worker does: one class loader in the test class path's order, so a service
  a project registers in its own resources is found
  ([the test verb](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md)).
- Non-transparent inline calls expand in a phase after the whole program is typed, as scalac's: an expansion's
  error is reported only when the typer reports none, and `--retain-trees` and `--max-inlined-trees` are scalac's
  `-Yretain-trees` and `-Xmax-inlined-trees`
  ([Macros, inline and derivation](https://github.com/Carrot-Inc/teq/blob/master/docs/COMPATIBILITY.md#macros-inline-and-derivation)).
- A build is the same at every worker count: its errors and warnings, their order and its exit, the class
  files, JavaScript and TASTy of a trait's `super` calls, and a JavaScript module's exports and the error
  `several entry points`, which follow the order in which the definitions of a file are written.
- A large program typed by several workers no longer aborts or is refused in some builds: on a `given T` import
  inside a class, a match-type alias reached through a type prefix, a member the JDK's class has and the standard
  library's declaration leaves out (`getClass.getResource`), or a file that waits for a macro of its own build.
- `teq compiler watch`, `teq lsp` and `teq fix` give memory back between builds: a session's resident memory on a
  large program is about half of what it was.

<details>
<summary>Conformance with scalac and the JDK</summary>

- Inline: a build that would hold an inline call that was never expanded is refused with an error at the call (it
  succeeded and the program failed when run); a local inline method called before its definition is checked in
  the context of its own place; an operator assignment through an `inline def` getter (`m.level += 1`) runs.
- Macros: a file whose expansion waits for a macro of its own build keeps what the expansion asked of a library,
  and a macro whose implementation needs a definition of its own build expands the same at every worker count;
  `Symbol.newMethod` with a `MethodType`, `Closure` over a method reference and `Select.unique` of a constructor
  expand as under scalac (tapir's `formCaseClassCodec`).
- Givens: a class made by an inline given's expansion reads the given's own recursive instance, and a given found
  through an inline given can refer to itself by name (kittens' `derives Eq` over a recursive enum); the
  divergence check of an implicit search compares the goals' covering sets as scalac does.
- Exports: a singleton type whose path goes through an export's forwarder has the exported member's value
  (`valueOf[Facade.width.type]`, a type test `case _: Facade.width.type`).
- Lambdas: an open parameter is typed from the method, constructor, function or enum case its body applies to it
  (`sink.contramap(Wrapped(_))`), a parameter spread into a repeated one as its sequence; a body that leaves its
  result's type constructor open takes that constructor's arguments as scalac does (`next.tap(_.traverse_(serve))`
  over zio and cats).
- The JVM: a `lazy val` is initialised once under concurrent readers and `synchronized` takes its receiver's
  monitor; a class lists as its interfaces the traits it inherits directly (a class below `javax.swing.JButton`
  loads); an unqualified `synchronized`, `toString` or `getClass` inside a function literal converted to a Java
  interface is the enclosing instance's.
- Java classes: a static member class of a Java class is no member of a Scala class below it (`Double` inside a
  class that extends `java.awt.Point`), and a library's class whose parent is a JDK class the standard library
  does not declare has that class's own parents and members.
- Enums: `valueOf` and `fromOrdinal` are taken as functions (`names.map(Tone.valueOf)`); every value of an enum that
  extends `java.lang.Enum` is a `Mirror.Singleton`; a case's `override val` parameter is accepted, and a case that
  names its enum's type parameters where it may not is refused with scalac's errors.
- Classes: a local class made through a second constructor that reads a local of the enclosing method, and an
  anonymous class made in a function literal applied where it is written (zio's `ZIO.succeed`), work on every
  target; a class's constructor code reads its own parameters, whatever a subclass overrides; a class whose type
  parameter takes a type constructor of its own (`class Later[F[G[_]]]`) is accepted from another file.
- The standard library: `List`, `Option`, `Either` and `Try` are abstract classes and `Iterator` and
  `immutable.Map` traits, as in scala-library; a sorted set's or map's `map`, `flatMap`, `collect` and `zip` keep
  their `Ordering`; `new Array[T](n)` with no `ClassTag[T]` is refused as scalac refuses it.
- Warnings and patterns: a pattern's binder over `T | Null` has the type `T`, and `-Wtostring-interpolated` reads
  an opaque alias as what it stands for (refined's `NonEmptyString`); `$"` in an interpolated string is a quote.
- JavaScript: a local named `$jsFn` or `$ap` no longer shadows the runtime's helper, and in split output a case
  object uses the `equals` of its superclass.
- `teq interp`: `ProcessHandle.of` takes its pid as the JDK does.

</details>

## 0.1.9 (2026-10-18)

- scalac's `-source` is teq's source version: the setting from the command line, `teq.lock`, a script's directives
  or the sbt and Mill plugins, then a unit's `import scala.language.3.x`; the migration table's diagnostics report
  by that version (`given` and `enum` as identifiers under `3.0-migration`, `with` as a type operator, implicit
  parameters without `using`, `x _`, alphanumeric infix operators, `import a._` and `a => b`, a `this` qualifier,
  vararg splices and wildcard types, non-local returns); a for comprehension's filter and the better fors follow the
  version, as do named tuples ([CLI](https://github.com/Carrot-Inc/teq/blob/master/docs/CLI.md)).
- A JVM build without a classpath takes the standard library the TASTy reader can read from the coursier caches,
  3.8.4 first; a cache holding only newer releases is refused with a message naming them
  ([libraries](https://github.com/Carrot-Inc/teq/blob/master/docs/LIBRARIES.md)).
- `teq interp` runs Scala CLI scripts: a `.sc` file wrapped as Scala CLI wraps it, its `//> using dep` resolved
  through the build's lock or the given `--classpath`; and programs with threads: `java.lang.Thread` is a logical
  thread of the interpreter with the JDK's park, interruption and monitors, `java.util.concurrent`'s locks, queues,
  executors and futures over it, zio's runtime among the programs that run ([runtime](https://github.com/Carrot-Inc/teq/blob/master/docs/RUNTIME.md)).
- `teq test` runs a build's suites on a pool as sbt's forked test worker does, one runner JVM per run; a Scala.js
  project's suites run under node or jsdom through the test bridge ([tooling](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md)).
- A Mill build exports its lock through mill-teq; sbt-teq is built for sbt 1 as well as sbt 2
  ([tooling](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md)).
- On Windows the daemon starts with a hidden console, so `teq compile` no longer opens a blank console window for
  each resident, generator and test runner.
- Warnings and deprecations are reported as scalac reports them: `-Wconf`, `@nowarn`, `-deprecation`, `-feature`
  and the `-Wunused` kinds ([CLI](https://github.com/Carrot-Inc/teq/blob/master/docs/CLI.md)).
- The language server's completion and signature help follow the presentation compiler's, and its index finds
  members as dotty does ([tooling](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md)).
- `teq interp` runs a hot loop or a repeatedly called body as flat instructions over an operand stack.
- `teq.cmd` reads the lock and maps coursier's cache as the POSIX launcher does; a download is retried with GET
  where a HEAD is refused.
- Every change is checked against lichess lila, Laminar, Airstream, Tyrian and Indigo, held to their recorded
  outcomes.
- A given search's divergence check measures its target as dotty does, so a program with wide typelevel tuple joins
  (tapir's endpoints) types a quarter faster than in 0.1.8 at eight workers and a third faster at one.

<details>
<summary>Conformance with scalac and the JDK</summary>

- A given's open type parameter stays open through a chained call until the expected type fixes it; a contextual
  lambda's result and an interpolated variable inside an enclosing instance stay open as dotty leaves them.
- A pattern definition's refutability is decided as `Checking.checkIrrefutable` decides it, so `val List(x) = ..`
  no longer warns; `Base64`'s MIME encoder and `UUID.nameUUIDFromBytes` on JavaScript.
- Givens are dotty's: searched through its contextual levels, an opaque type's companion in its implicit scope, a
  given object published before its body, `NotGiven` negated as `negateIfNot`, a trait's implicit object reached
  through an extending object, a member retried on its qualifier over dotty's argument cache.
- Inline: a library's inline bodies typed from its products, inline matches and erased values reduced as
  `InlineReducer`, an expansion's bindings counted as `Inliner.countRefs`, a written type tree kept apart from an
  inferred one in the pickle.
- Types and inference as dotty's: refinements as `RecType`, intersection members as `findMember`, nested classes'
  types through their prefixes, a literal typed by its constant type and widened by interpolation, an anonymous
  class's bound refined by its inherited alternatives, exhaustivity searched depth first in dotty's order, an
  erasure view kept beside the semantic type.
- Syntax and definitions as dotty's: constructor proxies through prefixes, copy and pattern holders as Desugar's,
  a case's pattern lexed in dotty's case region, an infix operand typed under its operator's selection, calls with
  defaults, using clauses and inline getters elaborated as one plan, local traits given their block's values as
  leading parameters and local enums their lifetime.
- The migration diagnostics as dotty's: an inherited accessor followed as `RefChecks.isInheritedAccessor`, infix
  checks as `Checking.checkValidInfix` on the resolved method, `x _` typed as `Migrations.typedAsFunction`, and a
  unit's warnings issued after its errors as the reporter's suspended messages.
- Checks as dotty's: members, access and patterns (E024, E120, E177 and their kin), exports through values and
  top-level forwarders, placeholders, conversions and overloads at lila's shapes, an inherited extension's
  parameter class seen from the receiver's prefix.
- Case classes and enums synthesized as dotty's; synthesized mirrors and companions written as scalac's
  `Synthesizer`; a nested class's constructor default getters in its companion; a quoted class's copy keeps every
  member in a module's products.
- The JVM target laid out, linked and erased as scalac's: case-class selectors and `canEqual` as members, `Any`'s
  members in the override checks, nullary methods; a member reference names the qualifier's class as the
  invocation's owner (JLS 13.1), so a package-private superclass of a jar's class works.
- The standard library follows scala-library's and the JDK's designs for views, partial functions, `ClassTag`s,
  `Objects`, regexes, reflection, fractional equality through the receiver's box, `BigDecimal.decimal` and
  `NumericRange`; it gained `java.nio`'s file channels and `walkFileTree`, `java.time`'s formatters,
  `JavaConverters`' decorators and `Class.getDeclaredMethod`; a sequence extractor's element type is read as
  `unapplySeqTypeElemTp` reads it ([compatibility](https://github.com/Carrot-Inc/teq/blob/master/docs/COMPATIBILITY.md)).

</details>

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
