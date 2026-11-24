# Changelog

What each release of teq changes for a user: the compiler, the build tool and the sbt plugin, the editor
support, and the platforms the binaries run on. The newest release comes first; its section is also the
notes of its GitHub release, which carries the binaries (`teq-<version>-<classifier>`, the Windows one with `.exe`) beside their
`SHA256SUMS`.

## Unreleased

- On the JVM an `if` or a `match` whose first branch is `null` where an `Any`, an `AnyRef` or an `Object` is wanted
  (`show(if c then null else s)` with `show(x: Any)`) is written so that its class loads; the verifier refused it
  ("Instruction type does not match stack map").
- A given instance's export clauses and their forwarders are in its class's pickle, as a class's are: another
  module built against the products calls the forwarders (`egh.Indented.h.f(1)` for `given h: AnyRef with { val c
  = Codecs; export c.* }`), where it was refused ("value f is not a member of h"). Exports in a given instance's body
  were right in a build of the sources together alone.
- A given that takes nothing and whose type is a path's singleton, an exported given's forwarder (`export
  Impl.given` in an object, a package object or a package's top-level file) or one written so (`given a:
  Impl.gi.type = Impl.gi`), is ranked against another given of its type as the given the path names, a structural
  one by its parents, as scalac ranks it: the two found by one search are scalac's ambiguity, where the other one
  was taken without a word.
- An import of a package's givens (`import fd.pf.given`, `import fd.pf.{given Ordering[Int]}`) brings the givens its
  package object exports (`package object pf: export Impl.given`, every kind of given, by `given`, `{given T}` or
  name), and so does the package's own code, chained package clauses' included, as scalac's does, where a build of
  the sources together found none of them and went on to the implicit scope: the program ran with another given. A
  package object read from another module's products had them.
- A generic method's call that leaves a default out after binding something ahead (an earlier clause's arguments,
  `def f[A](x: A)(y: A = x): A` called as `f(new V(2))()`, or its receiver, `(new K).f(v)`) gives a value class its
  own type on the JVM, as scalac: such a call at a value class `V` threw `ClassCastException` (`V` cannot be cast to
  `Integer`).
- An anonymous class of a trait over a class (`new T {}` for `trait T extends A`) passes `A`'s secondary-constructor
  arguments and a local `A`'s captured default, as scalac (JavaScript printed `undefined` or `0`; the JVM refused).
- On JavaScript and in the interpreter a named tuple's values are covariant, as under scalac (`NamedTuple[N, +V]`):
  `(a = 1, b = 2)` conforms to `(a: Any, b: Int)` wherever the two are compared, a file's first definitions and a
  build of several workers included, where it was refused ("Found: (a : Int, b : Int)"), or refused at one worker
  count and accepted at another. A type variable met only through a named tuple's values is inferred by their
  variance, as scalac's (`def mk[T >: Int](): (a: T => Unit, b: Int)`, then `val q: (a: Any => Unit, b: Int) = mk()`).
- A class's variance is checked through a named tuple, as under scalac: `trait Src[+T]: def put(x: (a: T, b: Int))`
  is refused ("covariant type T occurs in contravariant position in type (a : T, b : Int) of parameter x") on
  JavaScript and in the interpreter, where it was accepted, and an implementation called through a widened receiver
  threw a `ClassCastException` in the interpreter and gave another value on JavaScript. The JVM answered as scalac.
  A private constructor var and a `private[this]` parameter of such a type (`class C[+T](private var row: (a: T,
  b: Int))`) are refused as their `List[T]` twins already are, where scalac accepts both: they were accepted only
  while a named tuple's values had no variance.
- An `export` clause in a given instance's body (`given h: AnyRef with { val c = Codecs; export c.* }`, indented, in
  braces or after a colon; a wildcard, a selection, a renaming, a `given` selector) makes forwarders of the given's
  class, as a class's export clause does, where it was refused ("export clauses are not supported in a given",
  "exports are only allowed from objects and classes").
- A given alias with a using clause or type parameters defined in a block (`given g(using n: Int): String = ...`,
  `given g[A](using o: Ordering[A]): Ordering[List[A]] = ...`, an inline or anonymous one) is the local method scalac
  makes it, found by a summon, a call's implicit argument or another local given's using clause, where it was refused
  ("local givens cannot take parameters"). Two of them of one type are scalac's ambiguity, and two anonymous ones
  of one type scalac's double definition.
- On JavaScript, a class nested in another class (or in a local class, a local object or a trait) whose parent call
  is evaluated ahead of the parent's constructor (a default before a written argument, named arguments out of
  order, a secondary constructor's default) runs: the hoisted default getters and the hoisted written arguments that
  read the enclosing instance take it from the constructor's own parameter, as Scala.js's are, where they read it
  from `this` before `super` (a `ReferenceError` at run time) or, for an enum's case, from a field not yet set (a
  `TypeError`).
- An inline given whose using clause has a default (`inline given bark(using msg: String = "Woof!"): String`) has
  that default's getter as a method of its class (`Dog.bark$default$1`), as scalac's has: a call that leaves the
  argument out through an export of an instance or an import of its givens applies it, where it was missing (the
  interpreter's "has no default getters", a `NoSuchMethodError` on the JVM, a `TypeError` in JavaScript).
- An inline method exported by a class through a path of the instance (`class Holder: val c = Codecs; export c.*`,
  a trait's, an enum case's, a constructor parameter's, a nested path, a path through another export) and called
  with a default argument left out (`holder.m(3)`) evaluates the default on the holder, as scalac's call of the
  forwarder's own getter `holder.m$default$2`, where a build of the sources together read the path on the caller's
  own object (the interpreter's "Unit has no member c", a `ClassCastException` on the JVM, a `TypeError` in
  JavaScript).
- A generic inline method reached through an `export` of another module's object (`export lib.Codecs.*`, a
  selection, a renaming, an export of an export, an export inside a class) can be called by a third module built
  over the products: the forwarder's body keeps its type arguments, as scalac's (`Codecs.id[A](a)`), where it was
  withheld ("a call of id with 0 of its 1 type arguments") and the call refused. A method with an inline parameter
  that has a default stays withheld there, refused as before.
- A type test of `AnyRef` or `Object` is true for a boxed number, a boolean, a char and `()`, and false for `null`,
  as under scalac and Scala.js (`(5: Any).isInstanceOf[AnyRef]`, `case _: AnyRef`); in the interpreter it was false
  for every boxed value, and on JavaScript for numbers, booleans, `Long`s and `()`, where it was true for `null`.
- On JavaScript `()` is no `java.io.Serializable`, as under Scala.js, where its class extends `Object` alone
  (`().isInstanceOf[java.io.Serializable]`, a pattern, `classOf[java.io.Serializable].isInstance(())`); it was. On
  the JVM and in the interpreter it is, as under scalac.
- A jar that carries `teq-products.json` (a directory of products packaged as it is) is read by its number as the
  directory is: products of another number are refused with the products' line, their manifest named
  `<jar>!teq-products.json` (they were read as a library's classes, and a body that names a definition the reader's
  standard library no longer has failed with "not supported yet"). A session (`compiler watch`, and so the language
  server's and the project verbs' residents) reads products only from their directory, which it watches: one whose
  class path holds such a jar is refused at its start, naming it and what was run ("these products are packed as a jar,
  which `teq lsp` does not read: put their directory on the class path in place of <jar>"). A jar without that file, as `-o` writes
  it and the sbt plugin packages it, stays a library, never refused for its writer.
- A jar or products written by teq 0.1.11 or earlier whose methods take or return an intersection of value classes is
  to be rebuilt with this teq (their boxing and method descriptors were not scalac's); a products directory or a
  products jar of an older number is refused with the line that says so.
- A products directory or a products jar whose `teq-products.json` is there and cannot be read (not UTF-8, a read
  that fails, for a jar an entry failing its check) is refused with that file's line, as one that does not parse is,
  and so is a directory's damaged under a running session; a directory's such manifest was read as none, its products as
  plain classes with no number checked. The whole-module build (`--products <dir>` with the directory off
  `--classpath`) writes such a directory anew.
- A type test of `Any` or `Matchable` is false for `null`, as under scalac: `x.isInstanceOf[Any]`, a member of a
  union (`Any | String`), an inline body's test of a type parameter given `Any`, and `case _: Any` on a scrutinee
  typed `Any`. It was true on the JVM and in the interpreter, and on JavaScript outside a match.
- A macro sees `x.isInstanceOf[T]` and `x.asInstanceOf[T]` with the type the source wrote, once
  (`x.isInstanceOf[scala.Some[scala.Predef.String]]`; it saw `x.isInstanceOf[scala.Some[scala.Any]][scala.Some[scala.Any]]`), and a
  quote pattern over a type test matches the tested type as scalac's does (a test of `Int` is one of `AnyVal`).
- On the JVM a value of an intersection of value classes (`Meters & Feet`) is boxed as the class the intersection
  erases to, as under scalac (`Feet`, the first by scalac's order), where it becomes an `Any`, an element, an
  option's value, a receiver of `toString` and a `MatchError`'s value; it was boxed as the class written first, made
  of the other's underlying value. A method's parameter or result of three such classes whose first two sort the
  other way takes scalac's type (`def three(x: Meters & Feet & Grams)` takes an `int`; it took a `long`), so a
  library's method signature moves. Upcast to the class it does not erase to (`val s: VS = x` for `x: VS & VI`), such
  a value throws a `ClassCastException`, as the explicit cast does (scalac's class fails the verifier there; teq's
  did too). The products' number is raised for it (to 1; a source package object's products, below, raise it to 2):
  a module's products written by an earlier teq are refused on the class path, or rebuilt where they are the
  module's own, as an earlier teq's box of such a value is not this one's.
- A call of a value class's method initialises the class's companion object as under scalac, which calls the
  companion's extension method: before the receiver is evaluated, but for a generic class's receiver that is not
  idempotent (a call, a `var`, a def, a written cast), which is evaluated first; an object's val, a lazy val, a
  path through objects and an export's forwarder of a val are idempotent. On the JVM the companion was initialised after the receiver; in the interpreter and on
  JavaScript a companion whose body runs something was not initialised by the call at all.
- A program's Java sources are read for their declarations, as scalac reads the `.java` it is given: a directory
  gives its `.java` files with its `.scala` ones, a `.java` named on the command line is read as Java (it was
  parsed as Scala), and the Scala program sees each Java class and interface with its members, their types and
  parameter names, and a `static final` field's constant where the field is one literal (`static final int A = 1;`,
  not `1 + 1`). The Java sources' bodies are not compiled: on the JVM a build to a class directory writes the Scala
  program's classes and says so in one line, and javac compiles the Java sources against that directory
  (`javac -cp out:scala-library.jar -d out-java *.java`); a build to a jar, the JVM's default output, is refused, as
  the jar would lack the Java classes. On JavaScript and in the interpreter a call that reaches a Java source's
  method is refused, naming the source. The Java reader reads classes and interfaces: an enum, a record, an
  annotation declaration, a sealed type and a member type are refused where they are declared ("not supported
  yet"), a compact source file (members outside any type) at its first member, and so is a Java declaration that
  names a class of the same build's Scala sources; a Java body may name them. Names resolve by Java's rules, the
  JDK's packages known whatever was read before: an import whose qualifier names nothing is refused at the import in
  scalac's words ("value nope is not a member of <root>"), a name a qualifier does not hold where it is used ("Not
  found: type Nope", "type Missing is not a member of object p.Outer"), and a name an import claims is bound by no
  later scope; a lookup in a package finds a class the JVM holds there, never a type alias or the std's stand-in
  for a JDK class (`import scala.*` leaves `String` java.lang's). A simple type name in a Java class's members that a parent of the class declares as a member type of
  another class than the one its imports, its package or `java.lang` give (`Entry` beside `import q.Entry` in a
  class extending `AbstractMap`), which javac and scalac take, is refused at the name once a Scala source reads the
  class: this reader does not look up inherited member types. A `teq compiler watch` session and the language server read a Java source again when it changes.
- A Java source given with the program now defines its classes over a class file of the same name on the class
  path, as under scalac: a stale `J.class` on the class path beside `J.java` is no longer read. A Java and a Scala
  class of one name, and one class declared by two Java sources, are refused as scalac refuses them ("J clashes
  with class J", "J is already defined as object J"); until now a directory's Java source was dropped and the
  Scala class taken.
- A module's products carry the number of what they mean, the products ABI (`abi` in `teq-products.json`, 0 where a
  manifest names none, as 0.1.11's; `Teq-Products-ABI` in a jar `-o` writes; `teq --version --verbose` prints the
  binary's), and a teq reads only products of its own: a class path directory of another number is refused with one
  line naming both numbers and what to rebuild, a build over its own `--products` directory of another number
  rebuilds it keeping none of its entries, and the build tool empties a resident's class directory of another number.
  A whole-module build (`--products` with the directory off `--classpath`) removes every file the directory's
  manifest lists that it does not write again, whatever the number: a removed source's products no longer stay
  beside the new ones, unlisted, for a downstream build to read.
  A library jar is never refused for its writer: a jar is read as scalac's whoever wrote it. The sbt plugin, sbt-teq
  1.0.3 with this release, compiles a class directory of another number afresh, passes no upstream one of another
  number to teq, and leaves `teq-products.json` out of the jars it packages. This release's number is 2 (1, a value
  of an intersection of value classes boxed as the class it erases to, above; 2, a source package object's products,
  below): 0.1.11's products are refused, to be rebuilt with this release.
- A `package object p` is the object `p.package`, as under scalac: its members are the object's, written in
  `p/package.tasty`, `p/package.class` and `p/package$.class` (a member class `C` as `p/package$C.class`, the
  class `p.package.C`), no longer in the file's `p/<file>$package` (a member class as `p/C.class`), and the file's
  `<file>$package` holds the file's other top-level definitions alone. A file holding `package object p` beside a
  `package p` clause with top-level definitions wrote two products of one path, one of them lost (on the JVM a
  `ClassNotFoundException` at run time); a package object's class and an object of the package of one name shared
  `p/ID.tasty`. ``p.`package` `` names the object, and a member a definition of the package also has is "already
  defined", as scalac's. Products written by 0.1.11 hold the old layout: a module built with it must be built
  again, which the products' number (above) asks for. A second product of one path is now an internal error that
  names both sources.
- `teq interp` refuses a Java definition the run must execute and has nothing to execute for: no typed body and no
  native. It refuses where the run meets the definition, with the place in the sources and exit 3:
  `a.scala:6:11: error: the creation of Wide runs the constructor of the Java class jlib.Span, which does more than
  call its superclass's and store defaults`. Before, such a run went on without the Java code, often to a wrong
  value (a field the constructor sets read as 0, a static constant as null), or failed without a place. Inside a
  macro's run the refusal is the compile error at the expansion, in the same words.
  - A creation (a `new`, an object's construction, a function literal's class, an enum's value, a method handle's
    constructor) is checked once per class, in the interpreter's walk and in its lowered code alike. Each Java
    class among its superclasses must have no static initializer, and the constructor its subclass calls must do
    nothing but call its superclass's and store the defaults of its own fields. The class file read for this is the
    jar's or directory's entry the loader read, or the class in the runtime image (`lib/modules`) of the JDK the
    typing used. A typed class above a Java class must have a construction that does nothing. No Java interface the
    creation initializes (one that declares a method neither abstract nor static: a default, a private instance
    method) may have a static initializer.
  - A static field of a Java class that no native gives, read or written, is refused. So is a call the interpreter
    finds no member for that ends at a Java class's method or an interface's default no native gives. A Java class's
    own `toString`, `equals` or `hashCode` is not refused: the interpreter answers it with Object's, as before.
  - `java.io.File`'s `separator`, `separatorChar`, `pathSeparator` and `pathSeparatorChar` are the host's, as a JVM
    running the macro has them (`/` and `:` on Linux and macOS, `\` and `;` on Windows), where the std's JavaScript
    layer is not loaded (the JVM target, JavaScript over scala-library): a macro read them as null there, so munit's
    `Location` kept a file's whole path where scalac gives its path from the working directory.
  - A use that initializes a class (a creation, an object's construction, `Class.forName` with its initialization) is
    refused where the JVM's initialization of it (JVMS 5.5) runs a Java class's or interface's static initializer: a
    class's superclasses, then its superinterfaces that declare a method neither abstract nor static; an interface's own
    alone, as initializing an interface initializes none of its superinterfaces, so `Class.forName` of a trait over such
    an interface runs. `Class.forName("Shelf")` of `class Shelf extends jlib.Registry` skipped `Registry`'s initializer.
    A JDK class a native of the interpreter answers a member of is its own implementation, its initialization included.
  - Subclasses of `java.util.AbstractList` and `AbstractMap`, Java classes whose constructors store only defaults,
    zio's queues and the JDK objects the natives make run as before. A subclass of `java.awt.Point`, of a Java class
    whose constructor sets a field to anything but its default, or of an interface whose parent has an initializer
    and a method that is neither abstract nor static, is refused.
- An integer literal is read by the type expected, as scalac reads it: `val x: Long = 0xffffffff` is 4294967295 (it
  was -1), a `Long`'s digits past its range are refused ("number too large", never wrapped), `0b`/`0B` binary
  literals are read (`0b1010`), a separator at the digits' end is refused ("trailing separator is not allowed"), and
  a `Float` or `Double` expected reads decimal or binary digits as a floating number (`0b1010: Double` is 1010.0).
  A literal against a type variable is an `Int`'s, as scalac reads it (`(0xffffffff, 1)` for a `(Long, Int)`, `Some`,
  `List`, a generic method: -1), unless the variable is pinned to a class (`new C(0xffffffff)` for an invariant
  `C[Long]`: 4294967295); an overloaded operator's argument is an `Int`'s (`n + 0xffffffff` adds -1).
- A library's method body kept in its lean products (`--products`) is built over them with the arguments its compiled
  form holds: a `using` or implicit list it applies (written whole, short, named, or searched where the library was
  compiled), an extractor's implicit arguments in a pattern (`case E(n) =>` of an `unapply` with a using clause), and
  the alternative an overloaded call names. A program built over the products on JavaScript or in the interpreter
  searched those givens again in its own scope or chose another alternative: it printed another value (`101` for
  scalac's `8`), stopped with `has no member`, or was refused. A call of a method or a constructor of the lean std
  keeps the earlier reading.
- A trait's implicit clause applied with a short `using` list by an anonymous class or a class's parent
  (`new Tr(using 10) {}` of `trait Tr(implicit a: Int, b: Int)`) has its omitted trailing parameters searched as givens,
  as scalac does, where teq refused it ("missing argument for parameter b").
- The implicit conversion search builds the contexts a file's own import statements give once per file, place among
  its imports and kind of search, and keeps them while the program's scopes stand, instead of again at every search:
  a scope of many conversions checks 3 to 4% faster (the budget's view, gview and gvar programs, 20,000 conversions
  each). Which conversion a search takes, and every diagnostic, are unchanged.
- The Java reader reads enums, records, annotation declarations, sealed types and member types, where it refused them:
  an enum's constants, `values()` and `valueOf`, a record's canonical constructor and accessors, an annotation
  declaration applied to a Scala definition as one read from a class file, member types at any depth (static ones
  through their class's companion, `J.N`), and a simple type name a parent's member type gives, as javac binds it. A
  source with members outside any type (a compact source file) is dropped without a word, as scalac drops it. A match
  on a Java enum is checked for its constants, from a source or a class file (under `--std scala-library`, a match on
  `java.time.DayOfWeek` naming MONDAY and FRIDAY alone now warns "It would fail on pattern case: TUESDAY, WEDNESDAY,
  THURSDAY, SATURDAY, SUNDAY", where it said nothing), and a match on a sealed Java type of the build's sources for
  its subclasses among them; a sealed Java class of the class path is open to a Scala subclass and its matches are not
  checked by its `permits`, as scalac reads it (a Scala class extending one was refused, "Cannot extend sealed trait S
  in a different source file"). A static member class of a Java class that a library's TASTy names (`J.N`) has its
  members (it had none). A class file's member types are read at any depth too, where they stopped at eight levels
  (`J.N1. .. .N9` was "not found"). The analysis graph (`--analysis-version 3`) follows: a match on a Java enum of the
  class path takes the enum as a pattern-match target where the build names one of its constants, as scalac's does
  (it took none), and a match on a sealed Java class of the class path takes none (it took the class).
  (`J.N1. .. .N9` was "not found").
- A library's given summoned through scala-library's inline `summon` (`summon[Int]` of a jar's `given Int = 11`) is
  the given's value at every worker count, on the JVM and on JavaScript over scala-library: above one worker it was
  the type's default (0, `false`, `0.0`) where another worker had typed the given. A library's value read through an
  inline call (`summon`, a program's `inline def id[A](x: A): A = x` given a library object's `val v`, scala-library's
  `math.max(3, 7)`) is a call of the library's member, as scalac's client calls it, where it was the value the build
  read: a client run beside another version of the library prints that version's (`id(lib.Q.v)` printed the build's
  11 beside a `Q` whose `v` is 12). So is a String method of a literal or of a constant read through one
  (`id("ab".length)`, `id(Console.RED.length)`): the platform's method at run time, on the JVM and on JavaScript, as
  scalac's client calls it. A constant (`final val k = 5`) is folded as before.
- On JavaScript, `java.lang.Long`'s `highestOneBit`, `lowestOneBit`, `reverseBytes` and `toOctalString` run (they were
  refused: scala-library's `LongMap`, `hashing.byteswap64` and `RichLong.toOctalString` reach them), `Long.hashCode`
  is the JDK's (`Long.hashCode(-1L)` is 0, it was -1), and a number beside a `BigInt` or `BigDecimal` is equal as
  scalac's (`(42: Any) == BigInt(42)`, `Option(42).contains(BigInt(42))`, `List(42).contains(BigInt(42))`: true,
  they were false).
- A trait parent's or an anonymous class's `using` list that names its arguments and leaves a parameter out before a
  written one (`extends T(using b = ..)` of `trait T(implicit a: A, b: B)`) has that parameter searched as scalac
  searches it, where teq refused it ("missing argument for parameter a"). The arguments are evaluated once, in
  scalac's order (the written ones first, then the searched ones), a default reading an earlier argument included,
  where a trait of `using` or `implicit` clauses alone failed at run time. Every argument of such a trait is evaluated
  in the trait's turn of the class's initialization, after the superclass and the traits before it, as scalac evaluates
  it, where teq evaluated them all before any trait's body. A plain list given to such a trait's implicit clause
  (`Ti()(A(1), B(2))`, `T()()`) supplies its arguments and takes the defaults of the parameters it leaves out, never a
  given (with scalac's warning where a given is in scope), where teq dropped the list and took the givens. A trait whose
  `using` clause is followed by an implicit one has its omitted parameters searched, where teq refused it. Its lists are
  refused where scalac refuses them, in scalac's words: a list its constructor has no clause for ("does not take more
  parameters"), a plain list given to its implicit clause that leaves a parameter without a default ("missing argument
  for parameter b"), a plain list meeting the constructor's empty clause ("too many arguments"), and, for a trait of
  `using` clauses alone applied without its `()`, a named list that leaves a parameter out before a written one or whose
  arguments, out of their order, are bound first ("must be called with () argument"). teq accepted most of these. A
  library's kept body over its products whose local or anonymous class, or an enum's case, has a trait parent the
  products cannot carry (a call that binds its arguments first, or a case's second parent whose call binds an argument or
  whose trait runs a body) is refused there, naming it, where teq built the class without that parent's call.
- The TASTy a build writes places its trees where the parser read each token, as scalac's pickles place them,
  not where a search of the source's text finds it. A comment, a blank or a line break beside a token, a script's
  header, or a backquoted name no longer moves a tree's position. That covers a package clause, a variance sign, an
  anonymous class's `{`, a type selection's or a path's name, a `match`, a call's `(`, a lambda's arrow (the colon
  lambda's included), a `case` and a reference's written name (a backquoted package member's call is now an
  identifier, as scalac writes it). A tool reading the positions, and the language server's hover and definition
  over another module's products, see scalac's places: `(x: Int) /* => */ => x` puts its arrow at the real one, and
  `case /* c */ x =>` keeps its `CaseDef` at `case`. The `Quotes` a macro's reflection refers to is the one its
  parameter's written type, or its body's first import, names, a lambda's parameter included.
- On the JVM and in the interpreter a def's call of itself under a `return` is the def's loop wherever the `return`
  stands, as scalac makes it: in a statement, a `val`'s initialiser, a case, a `while`, a `catch` case or an operand,
  `final def f(n: Int): Int = { if n > 0 then return f(n - 1); 0 }` no longer overflows the stack there. Under a
  `return` in a `try`'s body (whatever its handlers), in a handler of a `try` with a `finally` or in a `synchronized`
  body the call stays a call (scalac's jump there skips the finalizer, a guard, a handler an outer frame would run, or
  the monitor's release), and `@tailrec` refuses such a def as scalac refuses a call not in tail position; so does a
  `return` in a local lazy val's initialiser or in an inline method's by-name argument, as in scalac. A call on another
  instance goes round the loop too, its receiver evaluated before its arguments, as scalac (the JVM ran the arguments
  first), a value class's included. On JavaScript such a call stays a call: a deep recursion there ends in a
  RangeError where Scala.js loops, and `@tailrec` refuses a def whose call of itself stands under a `return`, as
  before.
- On the JVM, nothing is written after a `return`, as scalac: a `return` where a reference is wanted (`val y: String
  = if x > 0 then return "p" else "n"`) no longer gives a class the verifier rejects (`VerifyError: Inconsistent
  stackmap frames`).

## 0.1.11 (2026-11-20)

- A module built for the JVM by an earlier teq must be built again: the descriptors of members over `X | Null` are
  scalac's now.
- An import through a value's member (`import shelf.row.*`) is no longer taken for another's at several workers (a
  wrong refusal, a wrong value or a crash).
- A macro's implementation that runs a method declaring a class, an `export`'s forwarder or a secondary constructor
  gives the same result at every worker count.
- A call of another module's `transparent inline` method whose body declares a class is pickled the same at every
  worker count; the class could lose its members.
- A build at several workers no longer stops with the compiler's own `index out of bounds` where an inline method
  reads a member of an `Array` or a `String`.
- A build at several workers no longer stops with `a wait on a cell under the loader's lock` (a nested trait, an
  intersection's overloads, a part rebuild).
- A module rebuilt in part over its own products is no longer refused with `recursive value alias needs type` where a
  given in them is typed by a rebuilt val.
- A build whose library data names a definition another worker is typing is typed again by one worker, with one
  worker's output, where it stopped
  ([the typer's workers](https://github.com/Carrot-Inc/teq/blob/master/docs/TARGETS.md#the-typers-workers)).
- A class in a method's body with a val beside an inherited method of the same name no longer stops the build with `a
  named method has its name`.
- A macro that uses a `java.util` map or set no longer fails in rare builds of a session kept open with several
  workers (`no builtin for java.util.Any.storeHas`).
- `teq fix` types a run's projects and runs their rules side by side, each in a process of its own; `--jobs n` bounds
  how many run at once
  ([jobs side by side](https://github.com/Carrot-Inc/teq/blob/master/docs/FIX.md#jobs-side-by-side)).
- `teq fix` says on stderr where it is and how it ended: a line as each job's typing begins and as its round ends, and
  a last line with the result and the time ([`teq fix`](https://github.com/Carrot-Inc/teq/blob/master/docs/CLI.md)).
- `teq compile` answers by the project asked for and says what the build did (`app: up to date`); `teq run`, `test`
  and `stage` name the project too
  ([what compile and run print](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md#teq)).
- `teq compile` and `teq run` name on stderr the build they wait for once it has not answered for a second.
- A command that meets a daemon of another teq binary or of a changed export starts the next daemon and says that it
  waits, where it gave up after three restarts
  ([the daemon](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md#teq)).
- A `--diagnostics` file holds one command's records, written when it ends; a build's outcome, a test run's counts and
  a failure's kind are records now.
- `teq test` no longer runs on a test JVM that kept an earlier invocation's environment: `FOO=2 teq test` after `FOO=1
  teq test` sees `FOO=2`.
- The editor offers "Import all missing names" (`source.addMissingImports`), one action that adds the import of every
  name of a file that has exactly one
  ([the language server](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md#the-language-server-and-zed)).
- In the editor, signature help and completion offer the constructors a place may call; at a case class's `V(`, the
  primary constructor alone.
- A type mismatch reads as scalac's, `Found:` and `Required:` on lines of their own, and a missing name `Not found:
  x`.
- A tool that reads a diagnostic's first line alone finds `Found:` there; the JSON's `message` field and the language
  server hold the whole message.
- A missing given reads as scalac's and names the callee: `No given instance of type T was found for parameter p of
  method m in object O`.
- A build no longer ends with `0 errors found` and a failing exit where an inline expansion reports an `inline val`
  that is no constant and then another error.
- A watch session and the editor show the errors of an inline method's or a macro's expansion as a fresh check shows
  them.
- A product's TASTy holds the body of a member first used inside an implicit conversion that is tried and given up; it
  was left out, or varied at several workers.
- A block's class whose own classes read the block's givens through inline methods has one constructor whether its
  module is built whole or in parts.
- The errors for the members a class inherits from two unrelated traits (`inherits conflicting members`) come in one
  order at every worker count.
- On the JVM a local enum's `valueOf` and `fromOrdinal` name the enum in their exceptions as scalac does at every
  worker count.
- An explicit argument to a context bound's evidence is reported under `-source 3.4` and `3.5` at every worker count,
  and through an export's forwarder too.

<details>
<summary>Conformance with scalac and the JDK</summary>

- Scala 3.6's given syntax is read as scalac's: a named given's condition (`given f: Int => String = ..`), conditions
  in parentheses, parents after a comma.
- An anonymous given object of several parents takes every parent's name, as scalac's does (`given_A_B`).
- A given type scalac does not read as one (a context function's, `1 | 2`, a bare match type) is refused, and so is a
  `using` written in a condition.
- In the given syntax before 3.6, a clause that is no using clause, an anonymous given of one bare type and a brace in
  the place of `with` get scalac's errors.
- A language import written from the root (`import _root_.scala.language.implicitConversions`) enables its feature.
- A brace block after a symbolic name selected with a dot (`xs.++ { .. }`) belongs to that name, as scalac reads it.
- An extension's methods are read from the indented lines after its header wherever the header's last clause closes (a
  header written over several lines).
- A line that begins with `.` after an indented block continues that block: at a `def`'s column it selects on the
  def's whole right side.
- An indented block of one expression is that expression: `for x <- xs yield` with `x` alone on the next line yields
  `xs` itself.
- A line that begins with `(`, `[` or `{` further in than its block is the argument list of the line before it; after
  a blank line it begins a statement.
- A `for` whose enumerators begin on its own line, without parentheses or braces, goes on over the lines after it up
  to its `yield` or `do`.
- The cases of a `match` or a `catch` and their bodies are divided as scalac divides them; a `case` on its `match`'s
  line without braces is refused.
- Indentation that mixes tabs and spaces is measured and compared as scalac does, and refused with its error where
  neither indentation extends the other.
- A type argument list after an expression in braces applies to the block's value: `{ poly }[Int]` calls the value's
  `apply`.
- After `return`, a block in braces followed by an argument list is refused as scalac refuses it.
- A type projection goes on over a line break after its `#` or before it.
- After `if (c)`, `while (c)` or `for (..)`, a line further in that begins with a bracket is the branch or the body
  itself and opens no block.
- Which of two givens is preferred follows the source version, as under scalac: up to 3.5 the more specific, and from
  3.7 a Scala 2 implicit compared the old way.
- Of two givens tied for an extension method, the method read on the more specific one is taken; the call was reported
  ambiguous.
- A given object of several parents (`given A, B:`) is found as each of them, from the program's sources and from
  products.
- An implicit conversion is searched as scalac searches it, from the innermost block out to the file's imports, a
  nearer candidate hiding its namesakes.
- An import brings implicits as scalac's does: a wildcard the `implicit` members alone, a `given` selector both kinds,
  within its type bound.
- An implicit conversion whose result is its own type parameter, bounded only above, gives a selection no member of
  the bound: `true.shout` is refused.
- An inaccessible protected member is left to an extension method or a conversion of the same name before the access
  is refused, as a private one already was.
- A written `using` list shorter than its clause has the omitted parameters searched as givens: `getJson(1)(using
  Config())` takes the `Translate` in scope.
- An overloaded method's alternatives are narrowed by a written `using` list's size first, and a default taken where a
  given exists gets scalac's E220.
- A case class whose first parameter clause is a `using` or `implicit` one is refused at its name (E144).
- A view or an extension method on a literal or a path takes the receiver's own type: `val o: Option[Stage] =
  "final".some` compiles for a union of constants.
- `--wunused imports` no longer calls `import p.*` unused where it brings a library's top-level extension or given of
  package `p` (ip4s's `ip"127.0.0.1"`).
- Under `--wunused imports` a selector that renames what a given search finds (`import p.{number as n}`) is no longer
  counted as used, as under scalac.
- `java.lang.String` written in full is the builtin `String` on every target; it was refused with "type String not
  found".
- An overriding member without a written result type takes the overridden members' result as scalac gives it: `def
  k(y: Any) = y` over `def k(x: Any): x.type`.
- On the JVM a trait's constant that overrides a member of a wider type (`override final val x = 1`) is bridged as
  scalac bridges it (an `AbstractMethodError`).
- A nested class holds its constructor parameters and type parameter bounds to the enclosing classes' variances:
  `class C[+A] { class Inner(x: A) }` is refused.
- A lambda's result that does not fit its type variable is adapted again once the variable is defined (`yield
  meter.tick()` in a `for` for a `Cell[Unit]`).
- A type parameter declared below `Unit` no longer takes any value and drops it: `f(_ => 5)` for `def f[B <: Unit](g:
  Int => B)` is a mismatch, as for scalac.
- A member declared `Unit` drops its body's value (`def paid: Unit = bump()`), as every other place that expects
  `Unit` does; teq kept the value.
- The warnings for a discarded value and for a pure expression follow scalac's one test of what is pure; a build with
  `--werror` can stop on one scalac gives too.
- A method named without its arguments where `Unit` is wanted is refused as scalac refuses it ("missing argument
  list").
- `==` and `!=` written as a selection (`k.==(k)`, `1.!=(2)`) are `Any`'s members, as the operators are.
- Under `strictEquality` a literal or stable identifier pattern asks for its `CanEqual` (`case Color.Red`), and a
  value of a bounded type compares by its bound's.
- Two values of a class that `derives CanEqual` compare where their type arguments compare pairwise.
- A type parameter whose upper bound is a refinement (`T <: AnyRef { def self: Int }`) conforms to a refinement that
  asks for that member.
- A class whose higher-kinded type parameter has a bound (`class Later[F[X] <: Iterable[X]]`) is accepted where it is
  applied before it is completed.
- A wildcard argument of a written type application is held to its parameter's bounds: `Up[? >: String]` for `class
  Up[T <: Int]` is refused.
- `scala.Dynamic` works on both standard libraries: a name a value of such a class lacks is its dynamic method's
  (`d.name` is `d.selectDynamic("name")`).
- A case class's default getter named in a source (`x.copy$default$1`) is no member, as under scalac.
- An assignment operator (`x += e`, `xs(i) *= e`) is typed first as a call of that operator, found as any member is,
  and only then as `l = l op e`.
- A compound assignment without such an operator evaluates its left side once, as scalac's typer arranges it: an
  `inline` parameter's argument ran twice.
- A protected member reached from a trait whose self type is its class, outside the member's package, is refused
  ("illegal access to protected method m").
- A creation chooses among the constructors its place can reach, as scalac decides it: a hidden secondary constructor
  no longer wins or gets the creation refused.
- A protected constructor is called as a subclass's parent alone and a qualified one (`private[p]`) inside `p` alone,
  a Java class's and another module's too.
- A case class's `apply` and `copy` carry the primary constructor's access, also in the products scalac compiles
  against.
- An anonymous class with an empty body (`new C(1) {}`) is held to what a subclass is: no final class, value class or
  another file's sealed class is extended.
- A protected constructor's default argument is taken inside the constructor's package alone, and a trait may extend a
  class whose constructor is private.
- A case class's name applied without `new` is its companion's `apply` alone: `V(1, 2)` no longer takes a secondary
  constructor `def this(a: Int, b: Int)`.
- A parent's arguments are typed outside the class they are written on, as scalac types them: a member around the
  class is that scope's, the class's own unseen.
- A constructor parameter named by an import inside a parent's argument (`extends Base({ import d.*; true.shout })`)
  is read as the constructor's argument.
- An object below a class nested in a class (`object Entry extends registry.made.Row`) is made with that class's
  instance, its path evaluated where Scala does.
- A constructor's `override val` parameter keeps its value over a parent's and gives way to a redefinition below it,
  on JavaScript and in the interpreter too.
- A class named through a path (`o.nums.Box`) has a `ClassTag`, its own `==` and `!=` are called, and its pattern
  against an unrelated class is refused.
- A class that joins a standard library method with a trait's method of the same name (`isDefinedAt`) no longer
  renames the library's method program-wide.
- A method that the override check refuses is reported once.
- On JavaScript a value class below a universal trait that overrides `equals` or `hashCode` compares and hashes by the
  trait's, as on the JVM.
- A value class keeps its own `equals` and `hashCode` where a trait's `equals` goes by another `@targetName`, and in a
  source under the `2.13` language import.
- The classes of a source under ``import scala.language.`2.13` `` are Scala 2's to the code that uses them, as under
  scalac (alphanumeric methods are operators).
- A Java class's static member class is a member of the class's companion alone, as scalac enters it: `this.Double`
  below `java.awt.Point` is refused.
- The operands of a right-associative operator (`a :: b`, `h *: t`) are evaluated in scalac's order: the left one
  first, before an object on the right is loaded.
- On JavaScript an object given to a `@js` template's method, or named after an argument a call binds ahead, is loaded
  where the arguments are evaluated.
- An object named as a statement is evaluated for its effect on every target; a statement that does nothing is removed
  where scalac removes it and nowhere else.
- A member of inferred result type, a default argument and an inline method are typed where they are defined,
  whichever use asks first, at every worker count.
- An inline method of inferred result type may call another written after it or in another file; two that call each
  other are refused as scalac refuses them.
- An inline expansion reduces `new C(args).x`, of a class that initialises nothing, to that argument, as scalac's
  inliner does: `inline if m.v > 0` reduces.
- An inline expansion reads a cast by the cast's own type: `n.asInstanceOf[Int]` is no constant, `n.asInstanceOf[9]`
  is `9` whatever stands under it.
- A transparent inline method reads an `inline` parameter's argument with the cast written on it:
  `flag.asInstanceOf[true]` selects an `inline if`'s first branch.
- An argument an inline method takes for its parameter is the caller's where a class of the method's body reads it:
  `over(this.n)` read `n` from the new class.
- A class declared in an inline method's body is a whole class at each call: the JVM stopped with
  `AbstractMethodError`, JavaScript called a wrong overload.
- A quote pattern's hole under a context function parameter binds an `Expr` of that context function type
  (`Expr[Selector ?=> a => tpe]`).
- A macro reads a class's nested classes and type members in the declared order, and `Symbol.typeMembers` holds its
  type parameters and what it inherits.
- The lean standard library's `Mirror` declares the type members scala-library's does: `m.MirroredElemTypes` of a
  plain `m: Mirror` is refused.
- A `for`'s alias with a written type (`for (y <- ys; l: T = e) ..`) is the value definition `val l: T = e`, not a
  pattern that tests `T`.
- A forward reference over a value's definition in a block is refused as scalac refuses it (E039), a typed pattern
  whose type a later class writes among them.
- On the JVM a lambda whose parameters are untupled from a tuple that holds a value class reads the element as the
  value class; it threw a ClassCastException.
- On the JVM `x.isInstanceOf[Array[String]]` looks at the array's element, of the class scalac's erasure names; it was
  true of any array.
- On the JVM a class nested in a class from the class path is constructed as its class file declares, with or without
  its enclosing instance (a `VerifyError`).
- `a ++ b` of two arrays runs in a lean JavaScript build; it stopped with `ClassTag$ is not defined`.
- `java.io.PrintStream` is the JDK's class on JavaScript and in the interpreter: open, over an `OutputStream`, with
  the JDK's members
  ([the standard library on JavaScript](https://github.com/Carrot-Inc/teq/blob/master/docs/COMPATIBILITY.md#the-standard-library-on-javascript)).
- `System.out` and `System.err` are such streams, `System.setOut` and `setErr` replace them, and `Console.withOut`
  reaches the prelude's `println` and `print`.
- A JavaScript program that prints through `System.out`, and one that prints under `--std scala-library`, grows by
  about 2.3 KB.
- `object Main extends App` builds with teq's own standard library, for JavaScript and in the interpreter: it declares
  `scala.App` and `scala.DelayedInit`.
- A name of teq's own standard library gets its deprecation warning wherever it is written, at every worker count,
  with scala-library's text and origin.
- A product's TASTy records the positions of a file's top-level wrapper, a package object, an extension group and an
  enum's cases as scalac records them.

</details>

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
