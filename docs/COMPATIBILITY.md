# Scala compatibility

Teq aims at compiling every Scala 3 program and running it as scalac 3.8.4 does: the expected output of
every test program comes from scalac, the Scala 3 compiler's own test suites run in the gate, and community
libraries compiled from their jars run against scalac's output. The remaining differences are listed below.

## The source version

scalac's `-source` is read where a build states it: teq's `--source` (`3.0-migration` to `3.9`, `future-migration`
and `future`, scalac 3.8.4's values, the others refused with its words, `2.13` and `3.1-migration` among them;
`3.8` by default), the sbt and Mill plugins' `-source 3.x` and `-source:3.x` among a configuration's `scalacOptions`
(the lock's `source` field of the configuration), and a script's `//> using options -source:3.x` under `teq interp`,
which a `--source` on the command line overrides as scala-cli's does; `teq compiler` reads no directive. A build of
several modules types each module's sources under its own version. A unit's source version import
(``import scala.language.`3.x` ``, or `future`) sets the unit's version as scalac's does: one per unit, at its top
level; the parser reads it from the import on (under `3.0-migration`, `given` and `enum` are identifiers from the
second token after the import), the typer for the whole unit, the definitions before the import included. Under a
`3.0-migration` setting indentation is not significant, whatever a later import names.

The version decides, as scalac's migration table does, whether a construct the language deprecates is accepted
silently, warned (from the version that deprecates it), warned as a migration (`migration warning`, at the
`-migration` version just before it becomes an error) or refused. At the default version these are refused: a
`for` generator whose pattern can fail without `case`, explicit arguments for a context bound's parameters, and
positional arguments of a Java annotation of a definition, but a lone one where the annotation has a `value` (the
std's `java.lang` annotations stand for the JDK's). Scala 2's syntax (`given` and `enum` as identifiers among it)
is read, with a migration warning, under `3.0-migration` alone. teq reports by the version, at scalac's places and
in its words: `with` as a type operator, `var x: T = _`, `_` as a type argument outside a match pattern,
`private[this]` and `protected[this]`, an argument's `xs: _*`, `f _`, an override of a `val` parameter by another
member than its inherited accessor, an alphanumeric method, type or extractor written infix and not declared
`infix` (but a backquoted one, one followed by a block not in parentheses, and a member of a Scala 2 class, scala-library's among them,
or of a library compiled before 3.4; a function value's `apply` is `Function1`'s, a polymorphic function value's
exempts nothing, `eq` and `ne` are declared `infix`), a non-local `return`, `_` and `=>` in an import's or an export's selectors,
the given syntax older than 3.6's, Scala 2's implicits (a clause, a definition, a lambda), an identifier holding `$`
(3.10.0's row, from 3.9), an infix operand that reads as named arguments, an assignment in parentheses or the named
tuple of one element it is from 3.7 (a migration warning at every version), explicit arguments for a context
bound's or an implicit clause's parameters, a pattern that can fail in a pattern definition or in a `for` generator
without `case` and, under `3.0-migration`, the Scala 3 keywords used as identifiers. A unit's warnings of its
parser and its typer come after its typer's errors, as scalac issues them, so an error hides a warning at its place.

The version changes what a program does where scalac's does: `f _` expands a method of any arity to its function
value, a method of `()` included, and refuses a value of no function type (`FunctionN`, `ContextFunctionN` or a type
that derives from `PolyFunction`, by its bound too; not a subclass of a function type, `=:=`) as Scala 2's syntax, which `3.0-migration` warns of and reads as
`() => x`, evaluated at each call; a `for` generator's pattern that can fail
is filtered before `3.4` unless its syntax shows it irrefutable (a tuple pattern over a value that is no tuple
literal is filtered, and so is a typed pattern, a stable identifier or a backquoted name in parentheses of its own,
`(x: Int)`) and checked from `3.4`; aliases are packed with the element by a `map` of their own before
`3.8`, and from `3.8` where an alias's pattern holds a given of its own (`(given Int, y) = ..`, `(given Int) = ..`);
from `3.8` a `case` generator's pattern is filtered again after each guard, and a trailing `map` of the generator's own
pattern is left out but after a guard of a generator without `case`, and where the receiver's type is not the call's
(a `map` typed `Any`, a conversion's receiver of another type) or an inline `map`'s expansion does not name its
receiver's binding; a generator's pattern in parentheses of its own (`(x)`, `(K)`, a stable identifier pattern) or
`_` is a case's whatever the version, and a typed identifier (`x: T`) the function's parameter at its type, a
mismatch where the element is no `T`; `(a = 1, b = 2)` is a tuple of
assignments before `3.7` and a named tuple from `3.7`, a named pattern and a named field's selection likewise; under a
migration version from `3.6` an infix operand of named arguments, or an assignment in parentheses, is the application
of them, without `-rewrite`.

## Not yet

### Syntax and definitions

- On the lean standard library, whose `Predef` gives `->` and its like as extensions rather than value
  classes, no view takes an `Any` to an `AnyVal`: an overloaded `unapply` with an `AnyVal` alternative,
  matched on a scrutinee typed `Any`, has no alternative in either round of the resolution, where scalac's
  `Predef.Ensuring` makes the `AnyVal` one applicable (as teq does on scala-library).
- A match on a sealed type whose cases cover it only together through one irrefutable sequence extractor
  (`case B() =>` and `case B(a, rest*) =>`, an `unapplySeq` returning a `Seq` or a `Some` of one) is warned
  as not exhaustive, where scalac's space of the extractor's sequence sees it covered; one case covering every
  length (`case B(all*)`) is not warned.
- `val Array(a, b) = s.split(",")` warns that the pattern's type is more specialized than the right-hand
  side's, which scalac does not.

### Types and inference

- On a receiver typed `Tuple` or `X <: Tuple` the members of `Tuple` are typed by their bounds (`head: Any`,
  `tail: Tuple`), and a named tuple has only `toTuple` and `size` of `NamedTuple`'s members on JavaScript and
  in the interpreter.

### Macros, inline and derivation

- Macro annotations (`MacroAnnotation`) and `scala.quoted.staging` are not covered; a reflection member the
  API lacks reports `not supported yet: scala.quoted <member> (called by <macro>)`. Of the 320 run-macros test
  programs of scalac 3.8.4's tag, 89 pass under node and 94 in the interpreter (41 of each have no check file
  and pass by running without an error).
- A macro reads teq's trees, not those dotty's typer leaves: an inline call within its argument carries no
  `Inlined(call, bindings, expansion)` node, a case class's `apply` is `new C(..)` and a mirror is made in place
  rather than bound to a proxy, so a macro matching those shapes fails (ultraviolet's `toGLSL`, which compiles
  Indigo's shaders: `ShaderError`s from `CreateShaderAST`).
- Plain inline calls expand in an inlining phase once the whole program is typed without an error, as
  scalac's: a program the typer reports an error in reports no expansion's, and a macro reading another
  member's definition sees its declaration (`--retain-trees`, scalac's `-Yretain-trees`, gives its typed
  right-hand side, the call unexpanded).
- Types carry no annotations (`AnnotatedType(t, annot)` is `t`, the type tree of a `T*` parameter is
  `Seq[T]`, so Magnolia's `repeated` answers `false`).
- An inline method of a library is expanded from its body as the library's TASTy holds it, checked once
  in its definition's scope as a program's own method is, so `constValue` of a parameter's singleton is the
  argument's constant and `codeOf` shows the typed argument, as scalac's expansion does (a by-value
  argument's proxy shown by the parameter's name). A body that check does not take is expanded by typing
  it again at the call, where a member only the argument's class has is accepted on the parameter: of the
  library bodies the suites expand (50 on the JVM, 27 of them scala-library's, and 23 on JavaScript), the one
  whose quote applies an inline parameter.

### The standard library on JavaScript

On JavaScript and in the interpreter a program and the libraries it loads run on teq's own standard library
(`--std=scala-library` compiles the real one instead); a member it lacks is reported rather than emulated,
a JDK member as `not supported on JavaScript`. Beyond that:

- `Locale` has no locale data: its display names are English, for the languages and regions its constants
  name, another code being its own name; on JavaScript the system zone is the engine's offset where the JVM
  names a region (the interpreter names the machine's zone, its `TimeZone` and `ZoneId` alike); in the
  interpreter a region zone id other than the machine's is not found, scala-java-time-tzdb building its data
  as JavaScript objects, which the interpreter does not run.
- `Class.forName` finds the program's own classes, by their binary names, which a reached call keeps in the
  output (a closed world: a class of the standard library or of a jar is not found), and array classes by their
  descriptors, whose component the same lookup finds; every class is the bootstrap loader's, so
  `getClassLoader` is `null` and a lookup through the `null` loader finds the program's classes, which the
  JVM's bootstrap loader does not.
- `java.time` comes from scala-java-time's jar on the classpath, region zone ids from its tzdb jar, and the
  system zone is the engine's offset; without the jar a build types the part of `java.time` the interpreter
  has (`docs/CLI.md`), which throws `UnsupportedOperationException` when run, as the file system does.
- A reflective call reaches the method whose parameters erase to the classes it gives, through the
  reflective proxy Scala.js names by them (a value class's argument and result its underlying value, as
  the JVM's method takes and gives it), and a missing one throws `NoSuchMethodException`; a receiver of a
  JavaScript type (a string, a number, an array) has no proxies, where the JVM finds the JDK's method.
  Under `--std=scala-library` scala-library's `Selectable` reflects as on the JVM, which throws
  `UnsupportedOperationException` on JavaScript.
- Regular expressions run on the JS engine behind a translator of Java's syntax, which has no possessive
  quantifiers, atomic groups, flags switched inside a pattern (`a(?i)b`), `UNICODE_CHARACTER_CLASS` (`(?U)`),
  the POSIX classes but `\p{Lower}`, `\p{Upper}` and `\p{Alpha}`, Java's own property names
  (`\p{javaLowerCase}`, `\p{IsAlphabetic}`) or `\G`.
- A `null` from an erased Java result of type `Int` (a missing key of a `java.util.HashMap[String, Int]`) is
  unboxed to `0` where an `Int` is consumed, as scalac unboxes it, but as the result of an inline method's
  expansion; `toString` and `hashCode` called on it give `"null"` and `0` (the interpreter's `toString`
  throws, as scalac's) where scalac throws `NullPointerException`.
- A collection class of a jar (scalajs-dom's `DOMListSeq`) answers `foreach` and what is built on it through
  its iterator, but not `map` and the operations that build a collection of its own kind, which the std builds
  through a member such a class lacks.
- Nothing blocks or runs in parallel: `Await.result` of an incomplete future throws `TimeoutException` at
  once, `synchronized` takes no monitor on any target, recursion has node's stack of about 10,000 frames
  (50,000 under Scala.js), there is no reflection, and `getStackTrace` is empty.

### Java and Scala.js interop

- `@JSExportTopLevel` in a jar is not exported, and `js.constructorOf[C]` answers native classes only.
- An exported member's alternatives (`@JSExport`) are chosen at run time by the number of arguments and the
  tests of their primitive and Scala-class parameters, as Scala.js chooses; a parameter of a Scala trait is not
  tested (Scala.js tests it), and alternatives no test tells apart are not reported, Scala.js's "Cannot
  disambiguate overloads for exported method".
- Reflective instantiation: every array is one class (`Array[Int]` and `Array[String]` erase alike), and a
  misspelt annotation registers nothing.

### The JVM target, sbt and warnings

- JVM: a build mixing the two compilers' modules meets these shapes, where teq's class files or pickles differ from
  scalac's: a trait's bridge over a generic parent's method sits in the trait, where scalac's is in each class
  mixing the trait in, so a class teq compiled against teq's trait throws `AbstractMethodError` over scalac's build
  of it; a method whose context function result a context closure gives keeps that result, where scalac takes its
  parameters into the method's (`ContextFunctionResults`); a union of a JDK class the standard library stands in
  for and another class erases to another class than scalac's (`Integer | String` to `Object` where scalac's is
  `ConstantDesc`, `StringBuilder | String` to `CharSequence` where scalac's is `Comparable`); a module's products
  carry no mixin forwarders, no static forwarders from a trait's companion, and a top-level opaque type's references
  in its companion's signatures and a renamed export of a `@targetName` member as scalac's reader does not take
  them.
- Under `-Wunused:implicits`, a using clause of an `extension` is one parameter its methods share, where
  scalac's is one per method: a method that does not use it warns in scalac, not here while another method
  uses it. Under `-Wunused:imports`, what a macro's or an inline method's expansion names uses no import:
  scalac resolves it at the call site, where a definition of the call site's own file counts as another
  file's, so an expansion naming one (a `derives` clause's macro) uses a wildcard import of the file's own
  package, which teq reports unused. `-feature` reports an implicit conversion's definition alone, not the use
  of a `Conversion` given or an extension of `scala.Dynamic`; on JavaScript the lean standard library
  deprecates none of its members, so a use of one scalac's deprecates (`mapValues`, `JavaConverters`) gives no
  deprecation warning there (`--std=scala-library` and the JVM read scala-library's). A warning at a place an
  earlier diagnostic took is hidden as scalac's reporter hides it, but an error is hidden only by the same
  error, where scalac's hides any at a place another error took: teq's errors do not all stand where scalac's
  do. A `@nowarn` whose argument names a definition local to the block around it registers nothing. Under
  `-Wunused:explicits`, a parameter of a method whose body has a constant type only as the union of its
  branches' literal types (`if (b) 1 else 1`) is reported: teq's typer gives a literal its widened type where
  scalac's gives it a constant type. A cast between primitives that no conversion makes warns where it is
  written but not where an inline method's expansion makes one, so a build with `-Werror` passes here where
  scalac fails it.
- A source version does not reach Scala 2's procedure syntax, an ascription after a pattern, an implicit lambda's
  parameter in parentheses or an XML literal, each of which scalac warns about or refuses by the version (its
  `MigrationVersion` rows). A named tuple's duplicate element name is not refused, and a vararg splice of a named
  argument (`f(x = xs*)`) is not read.

## Known differences

- `Double` prints as Scala.js prints it (`3.0` as `3`, `1e21` as `1e+21`, `-0.0` as `0`), a `Float` as the
  `Double` it is; `Int` and `Double` share one runtime type and `Char` and `String` are both JS strings,
  which only unions and `Any` see: an `Int` type test takes integral doubles, as a cast to `Int` does (an
  `Any` holding `7` passes a cast to `Double` or `Byte`, one holding a `String` a cast to `Char`),
  `('a': Any) == (97: Any)` is false, and an `Any`-keyed `Map` compares keys as JavaScript does (`1` and
  `1.0` one key, `1L` another). On JavaScript and in the interpreter strings and numbers are values, so `eq`
  of two equal strings or boxes is true, as under Scala.js.
- `hashCode` is `MurmurHash3` with Scala.js's hash of a `Double`; the unit is `()` in a concatenation and
  as `toString`, where Scala.js has `undefined`, which `println` and `print` show as Scala.js does;
  `(1: Any).isInstanceOf[AnyRef]` is false; `Option(x)` is `None` for `undefined` as for `null`; the bare
  term `String` is accepted, `String` and `Integer` being objects; on JavaScript a value class is never
  unboxed, equal as its field, and there and in the interpreter a cast to one tests the value where its
  result is dropped or widened too (scalac: where it is read as the class).
- `Map` and `Set` keep insertion order (scalac's `HashMap` orders by hash), and so do `java.util.HashMap` and
  `HashSet`, which a `toMap`, `groupingBy` or `toSet` result prints in; a `parallel` stream runs sequentially,
  JavaScript having one thread, and a stream of the file system fails on JavaScript, which has none; `TreeSet`
  and `TreeMap` are the classes of every sorted set and map; `x #:: xs` evaluates `xs` first (a self-referring
  `LazyList` is written `LazyList.cons(x, xs)`); `"abc".toSeq` is a `Vector[Char]`; a test or a cast against
  `Array[Int]` takes every array; a cast of a function type takes a function of any arity (a type test asks for
  the arity), since a JavaScript function's `length` is not its Scala arity and Scala.js's `js.FunctionN` are
  scala's function types here, where a cast to one is unchecked under Scala.js (a cast written to `js.FunctionN`
  is unchecked whatever its source). A `js.Array` converts to an immutable `Seq` where one is expected
  (Scala.js's `js.WrappedArray` is a mutable one, which scalac does not pass for it).
- `java.time` is scala-java-time's, from its jar on the class path, and the region zone ids its tzdb jar's,
  as a Scala.js build takes them, rather than a copy of the JDK's in the standard library.
- `Throwable` extends the native `Error`, and a value JavaScript throws reaches a `catch` as
  `js.JavaScriptException(value)`; the standard library's failures throw the JVM's classes with the JVM's
  messages, where Scala.js's differ; a member selected from `null` fails with the engine's `TypeError` (the
  JVM: `NullPointerException`); `null == undefined` is false; an uncaught exception prints node's stack trace.
- A val read before its initialiser ran is `undefined` (the JVM: `0` or `null`), so `val a: Int = a` fails at
  run time, and a superclass body reading a `val` the subclass overrides sees its own value until the
  subclass body runs, where scalac reads `0` or `null`; on JavaScript a concatenation renders an operand as its
  `+` runs, as under Scala.js.
- A call of a def through a top-level export does not initialise the exporting file, and on JavaScript and in
  the interpreter an export of a trait is no member of the classes below it (`needs to be abstract` where
  scalac accepts), a call being compiled as a call of the original.
- `Shape.Circle(1)` has the type `Shape` right away, where scalac widens the case type only when it infers a
  type from it (a member selected from it still sees the case). Accepted beyond scalac: `Codec[Int]` for a
  constructor `apply` that needs arguments, `xs.map(parse)` for a `parse` with a trailing default, a lambda's
  value discarded for a type variable bounded above by `Unit`, a macro defined and used in one compilation.
- A `lazy val (a, b) = (e1, e2)` whose tuple is written out evaluates each element once: a member's tuple at
  the first read of either variable, a local's element at the first read of its variable. scalac 3.8.4
  evaluates the element at every read (its `makePatDef` makes each variable a `def` of its element), a defect
  dotty fixed after 3.8.4.
- `println(())` and `print(())` print `undefined` on JavaScript and in the interpreter, as under Scala.js, and
  `()` on the JVM, as under scalac; a bare `println` prints an empty line on every target.
- JVM: a `null` from an erased Java result of type `Int` (a missing key of a `java.util.HashMap[String, Int]`)
  is `null` in a reference context and zero where `Int` is demanded, as under scalac, but an interpolation
  demands the `Int` (`s"${m.get(k)}"` is `0`, scalac's `null`);
  `(new W(Double.NaN): Any) == (new W(Double.NaN): Any)` is false for a value class `W`, where scalac's elision
  says true. Interpreter: `TimeZone.getDefault` is `UTC`; `UUID.randomUUID` is `math.random`.

## Left out on purpose

- `C.super`; name-based extractors (`isEmpty`/`get`); overloaded methods in a non-native JS class (Scala.js
  chooses between the alternatives at run time); `erased`, rejected wherever it appears; `AnyKind`;
  `scala.compiletime.testing.typeChecks`. `transparent trait` and `@throws` are accepted and change nothing.
