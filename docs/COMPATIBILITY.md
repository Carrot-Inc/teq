# Scala compatibility

Teq aims at compiling every Scala 3 program and running it as scalac 3.8.4 does: the expected output of
every test program comes from scalac, the Scala 3 compiler's own test suites run in the gate, and community
libraries compiled from their jars run against scalac's output. The remaining differences are listed below.

## Not yet

### Syntax and definitions

- A nested class made outside its enclosing class without a prefix (through an import of a value's members),
  and a constructor proxy through a prefix: `o.Inner(1)` is not found where `new o.Inner(1)` works.
- A local trait referring to the values of its block is `not supported yet`, a trait nested in a local class
  that reads them fails (at run time on JavaScript), and a local enum reading a value of the enclosing scope
  is `a local enum cannot refer to n: its values are made once`.
- A class nested in a generic class does not see the enclosing type arguments through a prefix (`o.Inner.get`
  on an `o: O[Int]` is an `A`), and `o.Item` and `o2.Item` are one type, their givens found through `this`.
- A wildcard or `given` import from a stable `val` of a package or object (`import dom.window.*`) reports
  the val as no object or package; a named selector works.
- A secondary constructor of a class nested in a class is given no enclosing instance, on every target:
  `new o.Inner()` through `def this() = this(2)`, or reflectively, fails where the class reads the instance.
- Confirmed, fix queued: `c.copy(a = 1)(2)` drops the second argument list, a local `lazy val (a, b) = e` is
  eager, an overloaded `unapply` is taken by its first alternative, a companion's own `unapply` is bypassed by
  a constructor pattern, and a by-name method eta-expanded without an expected type runs its argument once.

### Types and inference

- `val g: Int => F = F` for a case class `F` is rejected (scalac inserts `.apply`); `F.apply` works.
- `xs.foldLeft(List.empty)(...)` is rejected, `List.empty`'s type variable being solved before the lambda
  (`foldLeft(Nil)` works), and `val z: 1 = foo(1).zero` cannot take the literal type from the expected type.
- `<:<` and `=:=` evidence is filled in, never reasoned from (with `ev: A <:< String`, `a.length` is written
  `ev(a).length`); a type variable bound by a type test (`case l: List[t]`) is a wildcard the case cannot use.
- A match whose expected type is a match type gets no dependent typing of its cases (write the cast); on a
  receiver typed `Tuple` or `X <: Tuple` the members of `Tuple` are typed by their bounds (`head: Any`,
  `tail: Tuple`), `Tuple.Union` is not reduced, and a named tuple lacks `_1` and `NamedTuple`'s members.
- A failed member call is retried as an extension or a conversion of the receiver, as scalac retries it,
  except where the argument's typing failed inside a lambda, an `if` or a named argument, placed a warning,
  constrained a type variable of the call, spliced a sequence or was adapted; the member's error then stands.
- Exhaustivity: a match with a sequence pattern of fixed length (`List(a, b)`) or a stable identifier that is
  no enum case is not checked for missing cases, a type splits at most 64 times per match (beyond that nothing
  is reported), and a nested pattern that can never match is not reported; a type test of a trait over a class
  that is not final is warned unreachable (scalac: reachable, a subclass may mix the trait in).
- The variance check leaves private members out.

### Givens and implicits

- `import a.{given T}` brings every given of `a`, the type not filtering them; a given a block imports is
  nearer than one defined in an enclosing scope of the same method, where scalac keeps the definition.
- The implicit scope of an opaque type holds its companion where the type is transparent, which scalac's
  does not, and extension methods found through givens rank by their receiver alone, so an opaque type's
  companion instance loses to an imported low-priority one for its underlying type.
- A local `transparent inline given` is computed once, where scalac expands its right-hand side at each use;
  a program's own `given nf(using Missing): NotGiven[Int]` whose using clause fails is rejected, where
  scalac's negation makes it a success.
- A class that implements a `deferred` given by the search cannot have a method of its name with other
  parameters (`def x(s: String)` beside `given x: Int = deferred`): "only methods can be overloaded".

### Macros, inline and derivation

- Macro annotations (`MacroAnnotation`) and `scala.quoted.staging` are not covered; a reflection member the
  API lacks reports `not supported yet: scala.quoted <member> (called by <macro>)`. Of scalac's 322
  run-macros test programs, 69 pass under node and 71 in the interpreter.
- Types carry no annotations (`AnnotatedType(t, annot)` is `t`, the type tree of a `T*` parameter is
  `Seq[T]`, so Magnolia's `repeated` answers `false`), and `IntConstant` and the other kinds of constant are
  aliases of `Constant`, so `case c: IntConstant` matches every constant, the extractors telling them apart.
- An inline method of a library is expanded by typing its body again at the call, so a member only the
  argument's class has is accepted on the parameter inside the body, `constValue` of a parameter's singleton
  is no constant and `codeOf` gives the argument's source text; `erasedValue` used as a value is `()`.
- The mirror of a case object or a parameterless enum case is an object of its own
  (`summon[Mirror.Of[O.type]] eq O` is false), `fromTuple` and `fromProductTyped` are members of
  `Mirror.Product` rather than extensions, and `fromProductTyped` does not check the element types.

### The standard library on JavaScript

On JavaScript and in the interpreter a program and the libraries it loads run on teq's own standard library
(`--std=scala-library` compiles the real one instead); a member it lacks is reported rather than emulated,
a JDK member as `not supported on JavaScript`. Beyond that:

- `BigDecimal` and `BigInt` have no `until`/`to` ranges.
- `Locale` has no locale data, and the system zone is the engine's offset (the interpreter's `Z`) where the
  JVM names a region; in the interpreter a region zone id is not found, scala-java-time-tzdb building its
  data as JavaScript objects, which the interpreter does not run.
- `PartialFunction` cannot be extended, and `ClassTag` is a final class, so a program cannot define one with an
  overriding `unapply`.
- `BigDecimal` and `BigInt` have no `until`/`to` ranges, no construction from a `Float` or an `Array[Char]`,
  no `BigInt.probablePrime` or random constructors; `new java.lang.Integer(1)` is rejected.
- `java.time` comes from scala-java-time's jar on the classpath, region zone ids from its tzdb jar, and the
  system zone is the engine's offset; without the jar a build types the part of `java.time` the interpreter
  has (`docs/CLI.md`), which throws `UnsupportedOperationException` when run, as the file system does.
  `Locale` has no locale data; `Date.from` and `toInstant` are missing.
- `FiniteDuration` is the only `Duration`, `Ordering` covers tuples up to 5, `PartialFunction` cannot be
  extended, and `ClassTag` is a final class, so a program cannot define one with an overriding `unapply`.
- Regular expressions run on the JS engine behind a translator of Java's syntax, which has no possessive
  quantifiers, atomic groups, flags switched inside a pattern (`a(?i)b`), `UNICODE_CHARACTER_CLASS` (`(?U)`),
  the POSIX classes but `\p{Lower}`, `\p{Upper}` and `\p{Alpha}`, Java's own property names
  (`\p{javaLowerCase}`, `\p{IsAlphabetic}`) or `\G`. An invalid pattern given to
  `String`'s `matches`, `split` or `replaceAll` throws `IllegalArgumentException` where the JDK throws its
  subclass `PatternSyntaxException`.
- A `null` from an erased Java result of type `Int` (a missing key of a `java.util.HashMap[String, Int]`) stays
  `null` where `Int` is demanded: `val i: Int = m.get(k)` prints `null` (scalac `0`), and the interpreter's
  arithmetic on it fails.
- Nothing blocks or runs in parallel: `Await.result` of an incomplete future throws `TimeoutException` at
  once, `synchronized` takes no monitor on any target, recursion has node's stack of about 10,000 frames
  (50,000 under Scala.js), there is no reflection, and `getStackTrace` is empty.

### Java and Scala.js interop

- `@JSExportTopLevel` in a jar is not exported, and `js.constructorOf[C]` answers native classes only.
- Reflective instantiation: every array is one class (`Array[Int]` and `Array[String]` erase alike), and a
  misspelt annotation registers nothing.

### The JVM target, sbt and warnings

- JVM: `return` from a lambda is `not supported on the JVM yet`; a member whose `Nothing` or `Null` result is
  inferred erases to `Object` where scalac's descriptor names `Nothing$`, and teq's class files differ from
  scalac's in a listed set of shapes, so a module compiled by scalac does not link against teq's output.
- JVM, which links scala-library's bytecode: `Duration(1.5, SECONDS)`, `e.canEqual(x)` on an `Equals` and
  `sb.length()` of a `StringBuilder` are rejected where scalac types them, `f"n=$n%04d"` fails with
  `NoSuchMethodError`, a derived `Show` prints a case object as `Nil()`, `eq` on strings compares by value.
- A file's top-level `export` clauses are not written to a module's products.
- `-deprecation`, `-feature` and `-Wtostring-interpolated` have no counterpart, `-Wconf` is not read, the
  `-Wunused` kinds other than `imports` report nothing, `@nowarn("msg=...")` silences every warning of its
  definition, and a cast between primitives that no conversion makes warns where it is written but not where
  an inline method's expansion makes one, so a build with them and `-Werror` passes here where scalac fails it.

## Known differences

- `Double` prints as Scala.js prints it (`3.0` as `3`, `1e21` as `1e+21`, `-0.0` as `0`), a `Float` as the
  `Double` it is; `Int` and `Double` share one runtime type and `Char` and `String` are both JS strings,
  which only unions and `Any` see: an `Int` type test takes integral doubles, as a cast to `Int` does (an
  `Any` holding `7` passes a cast to `Double` or `Byte`, one holding a `String` a cast to `Char`),
  `('a': Any) == (97: Any)` is false, and an `Any`-keyed `Map` compares keys as JavaScript does (`1` and
  `1.0` one key, `1L` another).
- `hashCode` is `MurmurHash3` with Scala.js's hash of a `Double`; the unit is `()` in a concatenation and
  as `toString`, where Scala.js has `undefined`, which `println` and `print` show as Scala.js does;
  `(1: Any).isInstanceOf[AnyRef]` is false; `Option(x)` is `None` for `undefined` as for `null`; the bare
  term `String` is accepted, `String` and `Integer` being objects; `Math.max` and its kind are one
  definition over `Int | Long | Double`; on JavaScript a value class is never unboxed, equal as its field,
  and there and in the interpreter a cast to one tests the value where its result is dropped or widened
  too (scalac: where it is read as the class).
- `Map` and `Set` keep insertion order (scalac's `HashMap` orders by hash), and so do `java.util.HashMap` and
  `HashSet`, which a `toMap`, `groupingBy` or `toSet` result prints in; a `parallel` stream runs sequentially,
  JavaScript having one thread, and a stream of the file system fails on JavaScript, which has none; `TreeSet`
  and `TreeMap` are the classes of every sorted set and map; `x #:: xs` evaluates `xs` first (a self-referring
  `LazyList` is written `LazyList.cons(x, xs)`); `"abc".toSeq` is a `Vector[Char]`; `"a.b".split(".")` splits
  on the character, a `Char` and a one-character `String` being one value; a test or a cast against
  `Array[Int]` takes every array; a `Map`, a `Set` or a `Seq` is no function at run time (one is wrapped where
  a function is expected), so a type test of a function type is false for it and a cast to one fails.
- `java.time` is scala-java-time's, from its jar on the class path, and the region zone ids its tzdb jar's,
  as a Scala.js build takes them, rather than a copy of the JDK's in the standard library.
- `Throwable` extends the native `Error`, and a value JavaScript throws reaches a `catch` as
  `js.JavaScriptException(value)`; the standard library's failures throw the JVM's classes with the JVM's
  messages, where Scala.js's differ; a member selected from `null` fails with the engine's `TypeError` (the
  JVM: `NullPointerException`); `null == undefined` is false; an uncaught exception prints node's stack trace.
- A val read before its initialiser ran is `undefined` (the JVM: `0` or `null`), so `val a: Int = a` fails at
  run time, and a superclass body reading a `val` the subclass overrides sees its own value until the
  subclass body runs, where scalac reads `0` or `null`; a local enum's values are made once, shared by every
  run of the block; on JavaScript a concatenation renders an operand as its `+` runs, as under Scala.js.
- A call of a def through a top-level export does not initialise the exporting file, and on JavaScript and in
  the interpreter an export of a trait is no member of the classes below it (`needs to be abstract` where
  scalac accepts), a call being compiled as a call of the original.
- `Shape.Circle(1)` has the type `Shape` right away, where scalac widens the case type only when it infers a
  type from it (a member selected from it still sees the case). Accepted beyond scalac: `Codec[Int]` for a
  constructor `apply` that needs arguments, `xs.map(parse)` for a `parse` with a trailing default, a lambda's
  value discarded for a type variable bounded above by `Unit`, a macro defined and used in one compilation.
- `println(())` and `print(())` print `undefined` on JavaScript and in the interpreter, as under Scala.js, and
  `()` on the JVM, as under scalac; a bare `println` prints an empty line on every target.
- JVM: a `null` from an erased Java result of type `Int` (a missing key of a `java.util.HashMap[String, Int]`)
  is `null` in a reference context and zero where `Int` is demanded, as under scalac, but an interpolation
  demands the `Int` (`s"${m.get(k)}"` is `0`, scalac's `null`);
  `(new W(Double.NaN): Any) == (new W(Double.NaN): Any)` is false for a value class `W`, where scalac's elision
  says true. Interpreter: `TimeZone.getDefault` is `UTC`; `UUID.randomUUID` is `math.random`.

## Left out on purpose

- `C.super`; name-based extractors (`isEmpty`/`get`); exports that forward to a value (`export field.*`), an
  export path naming objects and packages; overloaded methods in a non-native JS class (Scala.js chooses
  between the alternatives at run time); `erased`, rejected wherever it appears; `AnyKind`; recursive
  refinements (`{ type T; def f: T }`); `scala.compiletime.testing.typeChecks`. `transparent trait` and
  `@throws` are accepted and change nothing.
