# Changelog

What each release of teq changes for a user: the compiler, the build tool and the sbt plugin, the editor
support, and the platforms the binaries run on. The newest release comes first; its section is also the
notes of its GitHub release, which carries the binaries (`teq-<version>-<classifier>`, the Windows one with `.exe`) beside their
`SHA256SUMS`.

## Unreleased

- A protected member reached from a trait whose self type is its class, outside the member's package, is refused
  as scalac refuses it: "illegal access to protected method m in class C from trait Side". It is refused by
  identifier, as a value, through `this`, inside a lambda, as a `protected[p]` member from outside `p`, and as the
  implicit conversion or given a search takes through the trait's `this`. No enclosing class can hold the accessor
  such a reference needs (scalac's ProtectedAccessors). teq built these.
  An enclosing subclass, an inner or anonymous class of one, a lambda inside one, the member's companion and its own
  package still reach it. So do the arguments of an anonymous subclass's parent call made in the trait
  (`new C(m(1)) {}`), and a member annotated `@scala.annotation.publicInBinary`, which needs no accessor.
  A reference scalac drops before that phase is not refused: an inline argument the expansion does not use, an
  untaken `inline if` or `summonFrom` case, a quote a macro leaves out, a call of a protected inline method, a
  constant condition's untaken branch (`if false`, a `final val`, `false && x`, `true || x`), and a constant val
  read through a path. A protected constructor's default argument dropped the same way is no longer refused either
  (`Ops.ignore(new Dflt() { }.a)` with an inline parameter it does not use): teq refused it.
  A `protected[p]` member is judged by its qualifier as the typer judges it, whether it is written in the sources,
  read from teq's products or from a jar, and whether the compiler made it (a deferred given's implementation, a
  case class's copy, an enum's member, an export's forwarder, an inherited member, a default): read inside `p` it
  needs no accessor, read through a self type outside `p` it is refused, in every one of those forms.
- A quote pattern's hole under a context function parameter binds an `Expr` of that context function type, as under
  scalac: `case '{ Field.const[a, tpe]($s) }` against `def const[A, F](selector: Selector ?=> A => F)` binds
  `s: Expr[Selector ?=> a => tpe]`, at any arity, by name and through an alias; it bound `Expr[a => tpe]`, and a
  program that states the hole's type was refused with a type mismatch.
- A member of no written result type that overrides takes the overridden members' result as scalac gives it, where
  programs scalac accepts were refused with a type mismatch: a result naming the overridden method's value parameters
  (`def k(x: Any): x.type` overridden by `def k(y: Any) = y`, a `c.T` of a parameter `c`) reads them as the member's
  own; the results of every base that declares the member are met (`AnyVal` from one trait and `Any` from another give
  `AnyVal`); an inline method's body is typed against the inherited result, which its calls take; a final val
  takes its right-hand side's type as adapted to the inherited one (a numeric widening, a conversion, the discarding
  into `Unit`: `override final val x = small` of a `Byte` against `def x: Long` is a `Long`), and keeps a constant
  over the inherited type (`override final val f3 = { println(3); 3 }` against `val f3: Int` is of type `3`).
  A trait's constant that overrides a member of a wider type (`override final val x = 1` against `def x:
  Matchable`) is bridged in the class that mixes the trait in, as scalac bridges it, and an object's mirror class
  forwards its getter (on the JVM the read through the overridden member threw `AbstractMethodError`, a written
  `final val x: 1 = 1` alike). A final val whose constant a block gives (`final val f = { println(3); 3 }`) is
  read as that constant in its own class: on the JVM `val g = f + 1` beside it failed with `NoSuchFieldError`.
- `java.io.PrintStream` is the JDK's class on JavaScript and in the interpreter: an open class over an
  `OutputStream` (`new PrintStream(out)`, with autoFlush, with a `Charset` or a charset's name), with the JDK's
  members: `print` and `println` of each type, `printf` and `format`, `append`, the `write`s, `flush`, `close`,
  `checkError`, `charset`. A program can extend it, and a subclass sees the calls the JDK makes: `println(x)` is
  its `print(x)` and then the line's end, the bytes go through its `write(bytes, off, len)`, a line's end under
  autoFlush flushes the stream under it; after `close` nothing is written and `checkError` is true. `printf` and
  `format` read the whole format before they write anything and write as the JDK does (a width's spaces one by
  one); a format the JDK refuses throws an `IllegalArgumentException` with the JDK's message. `System.out`
  and `System.err` are streams of that class (`new PrintStream(System.out)` writes to the process's output), and
  `System.setOut` and `setErr` replace them. `Console.withOut(stream)` now reaches the prelude's `println` and
  `print` too, which call the stream's own members, whichever stream it names, nested and restored after a
  throw. As scalac's, `Console` binds `out` and `err` at its first use (the prelude's first print is one): a
  `System.setOut` before it moves the prelude's prints and Console's with it, one after it moves neither, and the
  prelude's `println(x)` hands a stream the value itself (`println(Object)`); under `--std scala-library`
  Console's `out` and `err` are bound when Console is made, no longer at their first read.
  `FileOutputStream.getFD` gives a `java.io.FileDescriptor`. The class was final with `println`, `print` and
  `flush` alone, and `Console.withOut` left the prelude's `println` on the process's output. Not yet: the six
  constructors over a file's name or a `File` (write `new PrintStream(new FileOutputStream(file))`), `printf` and
  `format` with a `Locale`, a PrintStream as a `java.lang.Appendable`; on the JVM a class that overrides
  `PrintStream.printf` or `format` is refused (`not supported yet`). As scalac, a trait's deferred
  `printf(f: String, a: Any*)` is not implemented by the stream's (`needs to be abstract`), and a trait's
  `super.printf` finds no implementation in the class that mixes it in. A program that names neither the streams
  nor `Console` keeps its output's size, with some names spelled otherwise: the std's anonymous classes defined
  below the changed text of its files (they are named by their place there), and in a release build of a program
  that reaches `java.lang.System` some short member names (the release table counts System's two new members);
  the prelude's `println()` and `print(x)` are natives of their own (a program that calls `print` is 40 bytes larger,
  one that calls `println()` beside `println(x)` 34, `println()` alone smaller).
  One that prints through `System.out` grows by about 2.3 KB, as does every program under `--std scala-library`
  that prints (its `println` is scala-library's `Console.out.println`; with Console's `out` and `err` bound as it is
  made, 67 to 74 bytes fewer in a release build, 83 in a development one); one that also writes bytes to an
  `OutputStream` keeps the process stream's byte path too (7.5 KB in all for one of the benchmarks). One that calls
  `System.setOut` or `setErr` also carries what tells whether the prelude printed before Console bound its streams,
  and the stream's calls the prelude's prints then take: about 1.3 KB in a release build for `setErr` alone, 2.2 KB
  for `setOut` beside `println` (1.6 KB under `--std scala-library`, whose Predef never takes that route). Laminar's
  DOM definitions now generate under `teq interp` (the build's generator writes its files through a `PrintStream`
  over a `FileOutputStream`), so `teq compile` of Laminar runs the compile the generator's failure stopped, and
  shows teq's own errors there (702 when the generator first passed).
- An implicit conversion whose result is its own type parameter, bounded only above, no longer gives a member of the
  bound to a selection: with `implicit def conv[T <: Rich](b: Boolean): T` in scope, `true.shout` is refused ("value
  shout is not a member of Nothing"), as scalac refuses it, through an object's import, a value's wildcard or a local
  def alike. teq took `T` to be `Rich` and called the conversion. A lower bound or a using argument that fixes the
  parameter still gives the member, and the conversion is still taken for an expected type (`val r: Rich = true`).
- `==` and `!=` written as a selection are `Any`'s members, as the operators are: `k.==(k)`, `true.==(false)`,
  `1.!=(2)` and `"a".==("a")` compile and compare as `k == k` does, on a class deriving from `scala.Dynamic` too.
  teq refused them ("value == is not a member of K"), or took an extension method named `==` in scope. One argument
  that a class's own `==` does not take (`p.==(p)` beside a `def ==(x: String, y: String)`) is `Any`'s. Several
  arguments are tupled into `Any`'s one parameter where every `==` of the receiver takes one parameter and
  auto-tupling is on: `true.==("a", "b")` is refused as scalac refuses it ("values of types Boolean and (String,
  String) cannot be compared"), also where a conversion in scope gives a class with an `==` of two parameters,
  which teq took and called.
- A class nested in a class holds its constructors' parameters and its type parameters' bounds to the variances of the
  enclosing classes' type parameters, as scalac does (`VarianceChecker.relativeVariance`): `class C[+A] { class
  Inner(x: A) }` is refused ("covariant type A occurs in contravariant position in type A of parameter x"), and so are
  a secondary constructor's `def this(a: A)` and `class Inner[B <: A]`, which teq built. The enclosing classes are
  held up to the first whose owner is a term (a local or an anonymous class) or that is private to its instance, whose
  own parameters are still held; a private constructor and a local class itself hold nothing. A parameter or a bound
  is held only where its written type names classes and type parameters alone: one that names an alias, a path's
  member or `@uncheckedVariance` anywhere is not held. Under `-source:3.0-migration` it is a migration warning, as
  scalac's.
- A type mismatch reads as scalac's: `Found:    <type>` and `Required: <type>` on lines of their own, each note on lines of
  its own after them (implicit conversions not tried to `AnyVal` or `AnyRef`; the ambiguity of two conversions,
  `convert from A to B`, or of two extension methods, `provide an extension method`), then a match type's reduction
  note. A missing name reads `Not found: x`, a missing type `Not found: type X`. teq said `type mismatch: found T,
  required U` and `not found: x` / `type X not found`. A no-given message names a candidate's failure whole. A tool
  that reads a diagnostic's first line alone now finds `Found:    <type>` there: the JSON's `message` field and the
  language server hold the whole message.
- A class's own constructor parameter named by an import inside its parent's argument, or inside a default of a
  later parameter clause, is read as the constructor's argument, as the parameter's name itself is read there (a
  context bound's evidence, `[T: {Show as sh}]`, among them): `class Named(d: Dsl) extends Base({ import d.*;
  true.shout })`, whatever the import brings (a conversion, a given, a method, a val, an extension, a class's
  constructor proxy, a trait's members) and also from a closure, a local class or method made there that outlives
  the construction. teq read the parameter on a module of the class, which has none: the JavaScript build panicked
  or its run failed (`Nam$ is not defined`), the interpreter threw a NullPointerException and the JVM's class a
  NoSuchFieldError.
- The TASTy of a product (`--products`, and so the sbt plugin's and Mill's builds) holds the body of a member
  whose result type is inferred and whose first use comes inside an implicit conversion that is tried and
  given up: `x.foo(y)` where the nearer conversion needs a given the program lacks (an `Ord[Int]`) and a
  farther one serves, with `foo` of the first conversion's class written `def foo(b: A) = List(a, b).head`.
  Such a body was left out of the product. At one worker in a build of several files it was written with
  `Any` where the member's type parameter stood, which scalac refuses to read back. At several workers the
  product could differ from one build to the next. The program's output was right in each case.
- A build at several workers (`--threads`, or the count teq picks for a large program) could stop with the
  compiler's own "index out of bounds" in the place of its result, about one build in two of such a program:
  where an inline method's body calls a member that the platform gives an `Array` or a `String` (`a.length`,
  `a(0)`, `s.length`) and the program has enough inline calls for their expansion to take several workers. A
  build that completed was right, and one worker (`--threads 1`) never stopped. The same stop could take a
  `teq compiler watch` session at its second request, and stand in the place of a program's own error. What
  such a call records while an inline method is checked no longer outlives the workers that typed it.
- On the JVM a lambda whose parameters are untupled from a tuple that holds a value class reads the element as
  the value class: `pairs.map((code, text) => code.value)` with `Code(value: Int) extends AnyVal` threw a
  ClassCastException (`Code` cannot be cast to `Integer`), whichever parameter the body used.
- `teq test` no longer runs on a test JVM that kept the environment of an earlier invocation: the runner the
  daemon starts ahead for the next run is used only by a client of the same environment, so `FOO=2 teq test`
  after `FOO=1 teq test` sees `FOO=2`.
- `--wunused imports` no longer calls `import p.*` unused where what it brings is an extension a library declares
  at the top level of package `p`: ip4s's `ip"127.0.0.1"` and `port"8080"` with `import com.comcast.ip4s.*`.
  Such a definition is a member of its file's package object in the library, which the import's prefix now
  looks through, as scalac's does; so does a given such a package declares, imported by `p.given` or by its
  name and found by a search. A build under `--werror` stopped on the warning. A selector that renames what a search
  finds (`import p.{number as n}` with `summon[Long]`) is no longer counted as used, as scalac does not count it.
- Under `strictEquality` a literal or stable identifier pattern asks the evidence its comparison with the
  scrutinee needs, as scalac does: `case Color.Red` on an enum without `derives CanEqual` is refused ("values of
  types Color and Color cannot be compared"). teq accepted it; a build that compiled under teq failed under
  scalac. `import scala.language.experimental.strictEqualityPatternMatching` relaxes the check for an enum case
  or a case object as scalac's does.
- Under `strictEquality` a comparison of a value whose type is bounded by another (`x == "a"` with `x: T` and
  `T <: String`, an opaque type over `Int`, an intersection with `Int`, a `List` of a bounded member) takes the
  bound's `CanEqual`, as scalac does: its evidence is contravariant. teq refused them on JavaScript and in the
  interpreter.
- Two values of a class that `derives CanEqual` compare where their type arguments compare pairwise, as the
  instance scalac derives has it: `x == y` for `x: Box[A]` with `type A <: String` and `y: Box[String]`
  compiles. teq refused it.
- A lambda's result that does not fit the type variable it is typed against is adapted again once that variable
  can be defined, as scalac does it: `for { _ <- o; _ <- o } yield meter.tick()` for a `Cell[Unit]`, and
  `o.flatMap(_ => o.map(_ => meter.tick()))`, now compile, the value evaluated and dropped where the variable
  becomes `Unit` (with scalac's warning where the value is a pure one: `yield 5`). teq said "type mismatch:
  found Int => Meter, required Int => Unit". The same holds for a result that is a branch, a case, a handler, a
  block's end or a nested lambda, for `Unit` named through an alias or an intersection, and for a view to the
  variable's bound in scope, which is applied (`given Conversion[Meter, Unit]`). A type parameter DECLARED below
  `Unit` (`def f[B <: Unit](g: Int => B)`) no longer takes any value and drops it: `f(_ => 5)` is the mismatch it
  is for scalac, told at the lambda ("found Int => 5, required Int => Unit"), so a build that leaned on it stops
  there; so does a branch that does not fit before the one that would bound the variable (`if c then tick() else
  ()`), which no later branch recovers.
- A member whose declared type is `Unit` drops the value of its body, as every other place that expects `Unit`
  does: `def paid: Unit = bump()`, a `val`, `var`, `lazy val` or `given` of type `Unit`, an override and a
  local `def`, with the value the whole body or a branch of an `if`, a case, a handler or a block's end, and
  with `Unit` named through an alias. teq kept the value: JavaScript and the interpreter showed it
  (`println(paid)` printed `1`), and on the JVM a field of such a member failed when its class loaded
  (a VerifyError for a primitive, a ClassCastException for a `lazy val` or a `given`) or held `null`. A context
  function there is applied first and its result dropped (`def run: Unit = ctx` for `def ctx: Int ?=> Int` ran
  nothing and returned the function); without a given in scope that is now the error it is for scalac. Under
  `--semantic-types` a generic call whose result is dropped through an alias of `Unit` no longer has its
  argument turned into `()`, and an overload is no longer chosen by that result.
  The two warnings about a value nobody uses, "Discarded non-Unit value" and "A pure expression does nothing in
  statement position", now ask scalac's one test of what is pure, of a value the source writes as one: a
  literal, a name, a selection from such a value, a tuple, a function literal, a creation, a constant
  operation. They are told for more of what scalac tells: a value in each branch of such a member, `null`, a
  field's or a parameter's read, a value class's creation, an arithmetic constant; and no longer for what
  scalac passes: an inline call with an effect, an object with an initializer or named through another object
  or a package, `this`, and an ascription to `Unit` (the remedy the warning names: `(e: Unit)` was itself
  warned about). A call's result is not told, where scalac tells the few whose expansion does nothing
  (`summon[T]` of a parameter). The warning stands where scalac's does: at a creation's bracket, a selected
  name, a statement's operator. A build with `--werror` can therefore stop on a warning it did not have, each
  one scalac gives too.
  A method named without its arguments where `Unit` is wanted (`def run: Unit = step` for a
  `def step(i: Int): Int`; a lambda's result and a branch alike) is the error it is for scalac, "missing
  argument list for value of type Int => Int", where teq warned of a discarded value or said nothing. Only the
  name written there is refused: a function value that an inline call expands to at such a place, or that an
  applied function literal returns, is dropped as any value is.
- An object whose parent is a class nested in a class (`object Entry extends registry.made.Row`) is made with
  that class's instance, and its load evaluates the path to it (`registry.made`: `registry`'s load, a lazy val)
  where Scala does: after an argument that a call binds ahead of itself (on JavaScript), after a
  right-associative operator's left operand (on every target), and among a `@js` template's arguments in their
  order. The path was evaluated before the argument or the operand, and never where a template's text did not
  write the object.
- On JavaScript an object passed to a method written as a `@js` template (the std's intrinsics, a program's own)
  is loaded where the call's arguments are evaluated. A template that writes its second argument first
  (`@js("[$1, $0]")`) ran `swapped(Registry, step("b"))`'s `step("b")` before `Registry`'s load, and a template
  that does not write the object never loaded it, where that load runs an argument of a base's constructor (a
  class's default, a sealed class's) or a base's secondary constructor.
- The left operand of a right-associative operator is evaluated before the object on its right is loaded:
  `step("l") :: Registry` ran `Registry`'s load first, on JavaScript, the JVM and the interpreter, where that
  load runs an argument of a base's constructor (a parent's default, `trait Listed(n: Int = register())`) or a
  base's secondary constructor, in the program or in a jar: the operand's binding was left out beside every
  object whose own class runs no initialisation. It is left out only beside an object whose load runs nothing.
- On JavaScript an argument that a call binds ahead of itself is evaluated before an object named after it is
  loaded. Named arguments written out of order and a call that fills a default bind the argument written first
  (`pair(b = step("b"), a = Registry)`); the emitter wrote it back into the call behind an object it took to
  run nothing at its load, so an object whose load runs an argument of a base's constructor (a parent's default,
  `trait Listed(n: Int = register())`, a `using` argument, a context bound's evidence) was loaded before the
  argument was evaluated, and so was an object of a jar or of another module's products whose load runs a
  secondary constructor's body or a parent's written argument, which its pickle does not hold. The JVM and the
  interpreter evaluated them in the order written. An object is written back behind such an argument only where
  its class runs no initialisation and neither it nor a base takes a constructor parameter or has a secondary
  constructor.
- `scala.Dynamic`: a selection, an application or an assignment of a name that a value whose class derives from
  `scala.Dynamic` does not have, and that no extension method or conversion gives, is the class's dynamic method's:
  `d.name` is `d.selectDynamic("name")`, `d.name(a, b)` is `d.applyDynamic("name")(a, b)`, `d.name(x = a, b)` is
  `d.applyDynamicNamed("name")(("x", a), ("", b))` and `d.name = v` is `d.updateDynamic("name")(v)`, with the
  type arguments kept; so are a member the site may not access, a right-associative operator whose right operand
  is the value, `super.name` (where an extension method gives the name scalac calls the extension through
  `super`, a call teq does not make: the name is refused there as for a class that is no `scala.Dynamic`) and a
  stable identifier pattern through one (where no conversion gives the name: `case d.ensuring` is refused). teq
  had no `scala.Dynamic` on its lean
  standard library ("type Dynamic not found") and refused each of these against scala-library ("value name is not
  a member of D"). A class that derives from it needs `import scala.language.dynamics` (an error where
  `scala.Dynamic` is itself a parent, a feature warning otherwise); `applyDynamic` may not be overloaded in one;
  a name sent to a dynamic method the class lacks is refused with scalac's hint ("possible cause: maybe a wrong
  Dynamic method signature?"), which a type selected on such a value gets too; `js.Dynamic` is a `scala.Dynamic`,
  as in Scala.js, and its calls are what they were. A name every value has (`==`, `equals`, `toString`,
  `asInstanceOf` and the others of `Any` and `AnyRef`) is never a dynamic method's, whatever its form. An
  assignment operator is the application as written first, as for any class: `d += 1`, `h.cur += 1` and
  `xs(0) += 1` are `applyDynamic("+=")(1)` on the value where its dynamic method takes the call, and the
  assignment by the class's own `+` where it does not. Some forms are not sent to a dynamic method yet and are
  refused ("not supported yet: …"): an infix
  call with a parenthesised argument list (`d call (1, 2)`, one tuple to the dynamic method under scalac) and a
  written `using` argument list; and, on the lean standard library, `ensuring`, `formatted` and `→`, which
  scala-library's `Predef` gives every value before a dynamic method is asked, and every name on a value whose
  class also extends a class of the standard library (`extends Ordering[Int] with Dynamic`), which may lack there
  a member scala-library gives it.
- A name of a case class's default getter written in a source (`x.copy$default$1`, `x.m$default$1`) is no member,
  as under scalac: teq answered it with the field or the default.
- A language import written from the root (`import _root_.scala.language.implicitConversions`) enables its
  feature, as `scala.language.implicitConversions` and `language.implicitConversions` do. teq did not recognise the
  clause and warned that the feature should be enabled (an error under `--werror`).
- A protected member that the site may not access is left to an extension method or a conversion of the same
  name before the access is refused, as a private member already was: `new Guarded().stamp` from outside the class,
  with a `Conversion[Guarded, Rich]` whose `Rich` has `stamp`, calls the conversion's. teq reported the protected
  access and refused the program. Where an extension method of the name is not taken for the receiver (its using
  clause has no given, two imports bring one each, a type parameter's bound is not met) the access is what is
  reported, as under scalac, for a private member too, where teq told the extension's failure.
- An assignment operator (`x += e`, `q.x -= e`, `xs(i) *= e`) is typed as scalac types it: first as the call of
  that operator on the left side, the operator found as any member is, and only where that call does not apply as
  the assignment `l = l op e`. Three things were wrong before. An operator that comes from the type's companion,
  from a given, from a conversion or from an implicit class was passed over on a var, a field and an element:
  `var e = new E(0); e += 1` assigned `e + 1` where scalac calls the extension's `+=` (wrong values, silently).
  A program was refused where the left side has an `op=` that does not take the argument but an `op` that does
  (`+=(s: String)` beside `+(i: Int)`, `e += 1`), and where an applied left side names a method whose result has
  the operator beside an object of the same name (`data("id") := v`). And the element form, `xs(i) op= e` through
  `apply` and `update`, lost what the application as written has: an inserted default argument, named arguments,
  written type arguments (`c[Int](0) += 1`), a function literal typed by its parameter, `super.apply(i) += 1`, a
  written `c.apply(i) += 1`, and the alternative of an overloaded `apply` that was selected, whose by-name
  argument was evaluated early. What scalac refuses is refused now: an `apply` that is an extension's, one with a
  using clause, a structural type's, arguments named out of the parameters' order whose values are bound first,
  and `c(0) foo_= 1`, which was taken for an assignment through `foo_`. Where neither form applies the error is
  the call's (`value += is not a member of Int`, with the right side's own), no longer the assignment's
  (`value update is not a member`).
- A val that a class takes as a constructor parameter (`class A(override val v: String) extends P`) keeps the
  parameter's value whatever lies above the class and gives way to a redefinition in the body of a class below it,
  on JavaScript and in the interpreter as on the JVM. Two things were wrong before. Where a hierarchy mixes the two,
  the body lost or won at the wrong level: for `class B extends A("a") { override val v = "b" }`, `new B().v` was
  `a`, and under `class A2 extends P { override val v = "a2" }` with `class B2(override val v: String) extends A2`
  and `class C2 extends B2("b") { override val v = "c" }`, `new A2().v` was `P`'s value and `new C2().v` was `b`;
  so with a trait's val in `P`'s place, an abstract val, a generic parent, and a parent that takes the val as a
  parameter itself. And where the classes come from a library (another module's products, a jar scalac compiled),
  a parameter lost to the parent's own initialiser: for a library's `class K(override val t: String) extends T`,
  `new K("k").t` was the trait's value, and a parent's body that reads the val saw the wrong one. A class's own
  initialiser of such a val is now skipped exactly where a class more derived than it, in the instance's class,
  takes the val as a parameter. Not yet: a read of the val during the parent's constructor where a body below
  redefines it (scalac reads `null` there, teq the parent's value).
- `a ++ b` of two arrays runs in a lean JavaScript build. A program that concatenated two arrays, of any element
  type, stopped at that line unless something else of it had brought the `ClassTag` object in: `ReferenceError:
  ClassTag$ is not defined`, or a `TypeError` on `ClassTag$().apply` or on a tag's `wrap` where only a part of
  that object was there. The output evaluated the class tag scalac passes the concatenation, which the lean std's
  concatenation does not read. A tag the compiler makes for the concatenation is no longer evaluated there; one
  the program writes itself (`a.++(b)(using tag)`) is evaluated at each call, as it was. Where several expansions
  of an inline method share one function in the output, the tag is left out when every one of them passes a tag
  the compiler makes, and evaluated as before when one of them passes its own. A program whose concatenations ran
  prints what it printed; the tag is gone from the JavaScript of the call (`a.concat(b)` where the operands have
  no effects to order).
- A macro reads a class's nested classes and type members in the order the class gives them, as under scalac:
  `Symbol.declaredTypes` lists the type parameters, then the nested classes and the type members as the class
  declares them (for a Java class its member classes in the class file's order, then its type parameters; scalac's
  order of the member classes differs where the class file names eight classes or more, by its own table of them);
  `Symbol.declarations` lists the nested classes in the declared order after the terms; and `Symbol.typeMembers`
  lists what a selection on the class finds: the class's type parameters, then its own type members and classes
  and each parent's that are not private, a name once (of several parents that declare it, the one that defines
  it), the class's own first, then each parent's in turn, the class it extends ahead of the traits, and one
  class's by name (`class Tree[K, V] extends Ground with Leaf { class Nest2; type Moss2; class Nest1 }` gives `K,
  V, Nest2, Moss2, Nest1` and `K, V, Moss2, Nest1, Nest2` followed by `Ground`'s and `Leaf`'s). teq listed them in
  an order that moved with definitions elsewhere in the build (one more `val` in another file) and between a build
  of the sources and a build over another module's products, and `typeMembers` held the class's own declarations
  alone, without its type parameters and without what it inherits. A derivation that walks a class's nested
  classes now follows the declaration, and a library macro that pairs `typeMembers`' type parameters with a type's
  arguments reaches scalac's verdict (ZIO's `Tag` of a type with an intersection among its arguments,
  `Tag[List[Wax & Wick]]`, is refused with the library's message "You must not use an intersection type", as under
  scalac; teq derived a tag there). The lean std's `Mirror` declares the type members scala-library's does,
  `MirroredMonoType`, `MirroredLabel` and `MirroredElemLabels`: `MirroredType` and `MirroredElemTypes` are members
  of a mirror's refined type (`Mirror.Of[T]`) and of `Mirror.Singleton`, so `typeMembers` of a case class's
  companion holds neither, and `m.MirroredElemTypes` of a plain `m: Mirror` is refused in a lean build as under
  scalac (teq accepted it).
- On the JVM a type test against an array type looks at the array's element, by `isInstanceOf` as by a typed
  pattern: `x.isInstanceOf[Array[String]]` was true of any array (`Array(1, 2)` among them) where a pattern
  `case _: Array[String]` already told them apart; under a union or an intersection each member is tested by its
  own array class (`Array[Int] | Array[String]` was any array); an array of arrays of a type parameter is asked
  its two layers. Such a test is told uncheckable by what its element is, as scalac decides it:
  `Array[List[String]]` and `Array[G[Any]]` of an invariant `G` are, `Array[Box]`, `Array[List[Any]]` and
  `Array[G[?]]` are not (teq warned of every `isInstanceOf` against an array of a class); an opaque type's
  element is, outside the scope that sees through it. The element's class is the one scalac's erasure names: a
  value class or a universal trait with `Null` is an `Object` (`Array[V | Null]` is an `Object[]`, and a member
  whose signature names such a type now has scalac's descriptor, `Object[]` where it was `V[]`), a `*:` chain
  written out with `EmptyTuple` a `Product`, a refinement its parent, a structural type an `Object`, a polymorphic
  function type its function class. The descriptors over `X | Null` change with this (a parameter's, a result's,
  a field's), so a module built for the JVM by an earlier teq must be built again. JavaScript and the
  interpreter, whose arrays carry no element class, are as they were.
- A class named through a path has a `ClassTag`: `Array.empty[o.nums.Box]`, `Array(o.nums.Pt(1), o.nums.Pt(2))`,
  a context bound's `make[o.nums.Box](2)`, a class of a trait named through the object that inherits it
  (`summon[ClassTag[syntax.vec2]]`), the class's own inner object named inside the class, a projection
  (`Array.empty[Ops#Top]`), also as a part of a union, an intersection or an array, and `new Array[o.nums.Box](n)`
  on the JVM. teq refused each with "No ClassTag available for o.nums.Box".
- An inner class's own `==` and `!=` are called where the class is named through a path: `new o.nums.Money(1) ==
  new o.nums.Money(1)` with `def ==(o: Money)` answered `false`, the universal equality's answer. Not yet where
  the class is generic and an operand's type argument has to be inferred from the other operand
  (`c == new o.Cell(1, None)` for a `c: o.Cell[Option[Int]]`): that comparison is still the universal one. An own
  `==` that the site may not access (private, protected outside a subclass, private to a scope the site is
  outside) leaves the universal equality in place, also for a class named plainly, which teq refused with "== is
  private to C"; of an overloaded `==` the alternative that takes the operand is called (teq refused "values of
  types C and Int cannot be compared with == or !=").
- A typed pattern or a case class pattern of a class unrelated to the scrutinee's is refused where the classes are
  named through a path (`b match { case _: o.nums.Pt => … }` for a `b: o.nums.Box`), as it was for classes named
  plainly; teq built the match.
- An object named as a statement is evaluated for its effect on every target (`Registry` alone on a line, to run
  its initialisation). On JavaScript the object was left out where its own body runs nothing, so one whose
  initialisation is its superclass's (`object Registry extends Base("x")`: the superclass's body or a field's
  initialiser, a mixed-in trait's body, an argument of the parent's constructor) was never initialised; an
  object of a library named so was left out whatever it runs (of a jar on the JVM and on JavaScript, of another
  module's products on JavaScript); and the same went for a statement of a class body and for the receiver of
  `synchronized` on JavaScript. A statement that does nothing is now removed where scalac removes it and nowhere
  else: a literal, a local value, a function literal, or an object whose class runs no initialiser as scalac
  judges it, as a statement of a block; so the JVM and the interpreter no longer evaluate an object there whose
  only effect sits in a secondary constructor's argument or in an omitted default argument. Named through its
  package (`lib.Registry`) or as a statement of a class body such an object is evaluated, as by scalac. A class
  is judged as the build reads it: from its source where it is compiled, from its TASTy where it comes from a
  library or from another module's products. An object whose effect is an argument of its parent's constructor,
  named as a statement by its bare name, therefore runs in a build of both modules from source and not in a
  build against the other module's products, exactly as scalac differs between sources compiled together and a
  jar. What makes a class one that initialises is scalac's list: beside a statement, a `val` with a value, an
  object and a parameterless given, an `extension`, an export clause and two abstract vals written in one
  definition (`val a, b: Int`) do; a class of the JDK always does and a Java interface never, whatever the
  standard library writes for it, and so does a class of a Scala 2 library, a Scala 2 trait where it holds a
  concrete member. On JavaScript a block whose statements all do nothing is now written as its value would be
  written in its place: `{ (); () => 1 } != null` was no comparison of the function but a function itself
  (`() => 1 !== null`).
- Which constructors a creation may call is decided before its arguments choose among them, as scalac decides
  it: a constructor the place cannot reach is no candidate. Three things were wrong before. A hidden constructor
  could win: for `class C(val x: Any) { protected def this(x: String) = this("secondary") }`, `new
  C("argument").x` outside the class printed `secondary`, and with a `private` such constructor the creation was
  refused ("<init> is private to C"); both now take the primary constructor and print `argument`. A protected
  constructor is one a subclass calls as its parent's (`class D extends P(1)`, `new P(1) { .. }`, and now `new
  P(1) {}`, which teq refused), and nowhere else: `new P(1)` in the subclass's body, in its companion, in a
  class nested in it and among the arguments of its parent call was accepted and is refused as scalac refuses it
  ("constructor P cannot be accessed as a member of P from class D"). And a constructor with a qualifier
  (`private[p]`, `protected[p]`) of a class another module holds (a module teq built, for JavaScript or the JVM,
  and a jar scalac compiled) is read with its qualifier: inside `p` it is called (a `private[p]` secondary
  constructor was refused there, and on the JVM its class file had no such constructor, so a module scalac
  compiled against it failed with NoSuchMethodError), outside `p` it is refused (it was accepted for a class from
  a jar scalac compiled and for a `protected[p]` primary of any module).
  The same holds for what follows a constructor's access. A Java class's protected or package-private constructor
  is its package's (no subpackage's), a protected one a subclass's parent call's too: teq built such a creation
  from another package into a program that failed when it ran (IllegalAccessError, VerifyError), and between
  overloaded Java constructors it now chooses among those the place may call. `new java.util.logging.Level("X",
  5) {}`, an anonymous subclass of a JDK class through its protected constructor, is accepted (teq refused it)
  and runs on the JVM. A case class's `apply` and `copy` carry the primary constructor's access as members of the
  companion and of the class: in a subclass of `case class K protected (x: Int)`, `K(2)` and `K.apply(2)` are
  refused (the subclass is none of the companion), and `copy` is called on `this` or on an instance of the
  subclass, not on another `K`; and they are written so for a module scalac compiles against teq's products,
  where `K(1)` of a `case class K private[p] (x: Int)` was refused inside `p` and a protected one's was accepted
  in a subclass. A mirror (`Mirror.ProductOf[K]`) of a case class whose companion is a case object follows
  scalac's own rule: a subclass of a private-constructor class gets it (teq found none), the case-object
  companion itself and a place outside a qualified constructor's `p` do not (teq gave one). A trait may extend a
  class whose constructor is private (`trait T extends C`), since a trait calls no constructor (teq refused it);
  the class that then extends the trait calls it, and is refused where it may not (`class D extends T` outside
  `C`: teq reported the trait). A companion that extends its own class reaches the private constructor (`class C
  private (val x: Int); object C extends C(1)`: teq refused it). Where a class has several constructors and all
  are out of reach, the creation gets that one error and none about its arguments. In the editor, signature help
  and completion offer the constructors the place may call.
  One rule of scalac's that surprises is followed: a parent written without its type arguments (`class D extends
  G("s")`, `new G("s") { .. }` for `class G[T] protected (val x: T)`) is typed ahead of the class to find them,
  and there the protected constructor is out of reach: scalac refuses it, and for a class with a protected
  secondary constructor of two parameters takes the primary one with the two arguments as a tuple (`class E
  extends H("a", "b")` has `x == ("a", "b")`); with the type arguments written (`G[String]("s")`) the protected
  constructor is called. teq called it in both forms. The same holds where no argument list is written (`class D
  extends G0`, `object O extends G0`, `trait T extends G0`, `new G0 {}` for `class G0[T] protected ()`), unless the
  expected type gives a bare parent's type arguments (`val a: G0[Int] = new G0 {}`) or the place is inside another
  subclass of the parent, where scalac accepts the call typed ahead.
  An anonymous class written with an empty body (`new C(1) {}`) is held to what every subclass is held to: a final
  class and a value class are not extended ("anonymous class {...} cannot extend final class C"; `class D extends
  V(1)` of a value class `V` was accepted too and is refused), nor a sealed class from another source file, and
  the parent's self type has to be met ("illegal inheritance: self type C {...} of anonymous class C {...} does not
  conform to self type Need of parent class C"). teq built each of these into a program that ran; with a body
  written it refused the final, the sealed and the self-typed ones already.
  A protected constructor's default argument is taken inside the constructor's package (or what its qualifier
  names) and nowhere else, since its getter is a protected member of the companion: for `class Reg protected (val
  n: Int = 1)` of another package, `class D extends lib.Reg`, `object O extends lib.Reg` and `new lib.Reg {}` are
  refused as scalac refuses them ("illegal access to protected method $lessinit$greater$default$1 in object Reg
  from class D"); teq built them. With the argument written, or inside the package, they stand. The refusal is a
  later phase's for scalac, and is told as scalac tells it: once the program has no earlier error, and once for
  an application that leaves several defaults out.
  Not covered: an empty-bodied anonymous class of a class the lean standard library declares final (`new Some(1)
  {}`, `new java.lang.StringBuilder() {}`: accepted, as before), since the lean library's `final` is not always its
  library's; a self type that names a member of an enclosing class (`self: Outer.this.Dep =>`), left unchecked at
  an empty-bodied anonymous class; `private[this]` on a primary constructor, which teq reads as `private`; a creation through a
  restricted constructor in an inline method's or a macro's body, which scalac refuses at the definition unless
  the constructor is marked `@publicInBinary`; a mirror of a protected-constructor case class asked in a
  subclass of another package, which scalac refuses later ("Cannot use private constructors in inline methods");
  a constructor named as a function (`C.apply` or `new C(_)` where a function is expected), which still takes
  the primary constructor's parameters whatever its access.
- A compound assignment on a left side that has no such operator (`s.v += 1`, `xs(i) += 1`, `m.level += 1` over a
  getter and a setter) evaluates its left side once as scalac's typer arranges it (`typedOpAssign`): what the left
  side is selected on or applied to, and an indexed form's arguments, are bound ahead of the assignment unless
  they are a path whose evaluation does nothing. teq decided by another test. An `inline` parameter there was not
  bound where its method is defined, so its argument ran twice at each call (`inline def raise(inline s: Slot):
  Unit = s.v += 1` with `raise(make())` called `make()` twice; an indexed form's array and index and a getter and
  setter pair's receiver the same). An object with an initializer was read after an index with an effect
  (`Store(next()) += 1` ran `next()` before the object's initializer). An indexed form is now typed first as the
  application it is: where `f(i)` has the operator itself (`ledger(i) += x` on a result that defines `+=`), that
  call is made and nothing is bound; and an index passed to a by-name or an inline parameter of `apply` stays an
  argument of both `apply` and `update`. Programs whose operands there have no effects keep their meaning; the
  JavaScript and class files of a definition that assigns to a member of an object with an initializer, or
  through a written cast, gain a temporary.
- The arguments a class passes to a trait it extends (`class Mid extends Named(raise(9))` for `trait Named(val
  name: Int)`) are read from inside that class's constructor: a member of the object, class or trait around the
  class is that scope's. teq read such a member on the new instance itself, so the build succeeded and the program
  failed when it ran (JavaScript: `this.raise is not a function`, or `undefined` for a value; the JVM: a
  ClassCastException; the interpreter: `has no member raise`), for a class, an object and an anonymous class that
  extends the trait, with the member a method, a value, a `var`, a lazy value or one inherited, written alone or
  as `Outer.this.raise(9)`, as a `using` argument and inside a lambda; with a class between (a class in a class in
  the scope) the JavaScript named a local that was not there and the JVM build was refused. A trait whose type
  arguments the argument infers (`extends Boxed(raise(9))`) was right already.
  A parent's argument, a class's as a trait's, is now typed as scalac types it, outside the class it is written
  on, and so is everything the argument defines (a lambda, a local method, a block's class with its own parent
  call). Four things change with that. A member the class declares no longer shadows a name around it: `object
  Mid extends Base(size) { val size = 3 }` beside an outer `size` read the object's own value before it was set
  (`0`; a VerifyError on the JVM). A type or an import of the class's body is not found there: `class Mid extends
  Base(new Part) { class Part }` and `extends Base(helper(1)) { import Tools.helper }` are refused as scalac
  refuses them ("type Part not found", "not found: helper"); teq accepted both. `this` in such an argument is the
  class around, as for scalac (`class Mid extends Base(this.tag)` inside a class or an object): teq refused it in a
  class parent's argument and read the new instance in a trait parent's; for a top-level class it stays an error.
  Not covered: `super` in a parent's argument, which scalac reads as the class around's and teq refuses.
- A case class's name applied without `new`, and its `apply` written, named as a value or reached by a
  placeholder, is its companion's `apply` and nothing else: the primary constructor's parameters, with the primary
  constructor's access. teq took a secondary constructor there too, as a plain class's name does. With
  `case class V(x: Int) { def this(a: Int, b: Int) = this(a + b) }`, `V(1, 2)`, `V.apply(1, 2)`, `V(a = 1, b = 2)` and
  `V(_, _)` compiled where scalac refuses them, and where the primary's one parameter takes a tuple the program
  printed another value: `G(1, 2)` of `case class G[T](x: T)` with `def this(a: T, b: T)` is `G((1,2))` (teq printed
  `G(1)`), and so for a parameter of type `Any`, `Product`, a tuple type or a union with one, and a repeated one.
  Arguments that fit the primary and a secondary constructor of the same arity are the primary's too, where teq
  took the more specific secondary: with `case class A(x: Any) { def this(s: String) = this(s.length) }`, `A("abc")`
  is `A(abc)` (teq printed `A(3)`); `L(1)` of `case class L(x: Long)` beside `def this(i: Int)`, and `Df(1)` of
  `case class Df(x: Int, y: Int = 10)` beside `def this(a: Int)`, are the primary's `L(1)` and `Df(1,10)`. A case
  class with a second parameter list applies with tupled arguments as any other (`Cc(1, 2)(3)` of
  `case class Cc(x: Any)(val y: Int)` with a secondary constructor was refused). A
  private primary constructor's `apply` is private too: `val f: Int => V = V.apply` outside the companion and the
  class is refused. The same for a case class read from a class path (a jar, another module's products): applied by
  its name, through an import or an export, nested in a trait and reached through an object or an instance that
  inherits it, or named in the arguments of a `new` of an array. New refusals, scalac's: every such application with a secondary constructor's arguments (teq's words
  for them are "too many arguments: expected 1", "there is no parameter named a", "the constructor of V is private to
  V"). A `new`, a parent call (`class Sub extends V(1, 2)`) and an anonymous class (`new V(1, 2) {}`) reach every
  constructor as before, and so does a plain class's name. A placeholder creation under an open type parameter
  (`twice(V(_), 3)` for `def twice[A](f: A => V, a: A)`) takes the primary's parameter type, where teq refused it for a
  case class with a secondary constructor. In the editor, signature help at `V(` offers the primary constructor
  alone; at `new V(` every constructor. Two programs are refused until tupled arguments are adapted as scalac
  adapts them: `V(1, 2)` with a `Conversion[(Int, Int), Int]` in scope, and `V(a = 1, b = 2)` for a parameter of type
  `Any` (scalac tuples the named arguments). teq printed scalac's line for them only where a secondary constructor
  happened to compute it; without one it refused them already.
- On the JVM, a class nested in a class that comes from the class path (a jar, another module's products) is
  constructed as its class file declares the constructor: with its enclosing instance first where the constructor
  takes one, without it where it takes none. A class that extends such a class with arguments (`class Level(i: Int,
  name: String) extends Val(i, name)` inside an `Enumeration`; `class Sub(k: Int) extends api.Request(k)` over another
  module), an anonymous class of one, and a creation of one whose constructor is overloaded were built and then
  failed at class load with a `VerifyError`; a creation of a class that scalac gave no enclosing instance (a class
  private to its owner that reads no outer) failed with a `NoSuchMethodError`. An anonymous class through a secondary
  constructor of a class nested in a class (`new api.Second() { .. }`) failed the same way in a whole build too. The
  class file's constructor is the one whose parameters are the selected constructor's, never one that only has as
  many.
- An inline method's expansion reduces a projection as scalac's inliner does: `new C(args).x`, the creation of a
  class that initialises nothing (a case class's `C(args)` too) selected for one of its parameters, is that
  argument. It is so where the selection is the whole right side of a parameter's or a receiver's binding
  (`pick(new Mid(1).v)`) and where the method's body selects on a creation of its own or on an `inline`
  parameter's argument (`inline def pick(inline m: Mid) = m.v`), and what a reduction gives is reduced again; the
  creation's other arguments keep their effects in their order. For a program: the instance is no longer made.
  Programs that now compile or print scalac's line, the argument's constant being read: `inline if m.v > 0`
  (teq: "Cannot reduce `inline if`"), a transparent method whose result is such a selection
  (`val one: 1 = pick(new Mid(1))`) and `inline m.v match` (teq took the other case). New refusals, scalac's
  (E039): a forward reference to a block's later class through a creation the inliner keeps, which teq accepted
  inside every expansion: a selection inside an argument (`pick(new Mid(1).v + 1)`), in a lambda, in the argument
  of a plain call, under an ascription, and a by-name parameter's argument. Kept, as scalac keeps them: a by-name
  parameter's argument, a by-value parameter bound to the creation, a value of the body, a class with a second
  parameter list, with a value or a statement in its body or with a parent applied to an argument, and a
  parameter the method reads through an accessor (a private one outside a final class). Not ported, and
  temporary: four shapes in which scalac's own reduction gives another value or loses an effect (a case class
  with two parameter lists, a creation through a secondary constructor, a class whose parent takes a default
  argument, a parent applied in two clauses): teq makes the instance and prints what it printed, and the check
  of forward references still accepts a later class created in one of the first three inside an expansion, as
  scalac does. Two more places where teq does what the program means and scalac does not: an assignment to a
  `var` parameter of such a creation in the method's body is made, as outside an inline method (scalac refuses it
  there: "Reassignment to val"), and a class's by-name argument is not run where a reduction drops the creation
  (scalac runs it once for each reduced selection). Not covered yet: the components of an `inline match` on such
  a creation.
- The operands of a right-associative operator (`a :: b`, `a +: b`, `h *: t`, a method `op:` of one's own) are
  evaluated in scalac's order: the application `b.op(a)` is typed as any application and each of its completed
  arguments that is no pure expression, for a parameter that is neither by-name nor inline, is evaluated ahead of
  the receiver, as the reference's typer lifts it (`Typer.typedInfixOp`). teq decided at another place by another
  test, so the right operand ran first when it was a top-level or local lazy value, a top-level given, an object
  with an initializer or an inline parameter (`left() :: lazyList` ran the lazy value's initializer before
  `left()`); for every left operand of a tuple's `*:` (`item(1) *: item(2) *: EmptyTuple` ran `item(2)` first); for
  an operand passed as an auto-tupled pair, converted by an implicit conversion, or beside a receiver a conversion
  wraps; and in a whole build of an inline method that conses a `summonInline` onto a call (a derivation made the
  last field's given first, where a build over the method's products made the first's). The argument of an inline
  parameter is no longer evaluated once ahead of the call, it is evaluated where the body reads it. A pure operand
  is no longer bound to a temporary (`p.name :: rest()` reads `p.name` after `rest()`, so with a null `p` the
  receiver's effects come before the exception, as with scalac), and a cast the source writes is evaluated ahead of
  the receiver. Beside a receiver whose evaluation can do nothing (a local value, a parameter, `this`, an object
  without an initializer that is not read from another module's products) the operand runs at the same point bound or not, and no temporary is written; an inline
  method's body binds the operand whatever its receiver and each expansion keeps or drops the temporary by what
  the receiver has become there (a build over the method's products keeps it where the whole build drops it: the
  same order, another text). The pickled body of a right-associative extension method's application now also
  binds a left operand that is a lazy value, an inline or by-name parameter or an object with an initializer.
  Programs whose operands have no effects keep their meaning; their JavaScript and class files lose or gain a
  temporary in the definitions that hold such an application.
- A `for`'s alias with a written type (`for (y <- ys; l: T = e) ..`) is the value definition scalac makes of it,
  `val l: T = e`, where its run of aliases is not followed by a guard and where it stands before the first
  generator; teq typed it as a pattern that tests `T`. Three things change for a program. Programs that now
  compile: the right side is typed against `T`, so a numeric widening (`l: Long = y`), a lambda's parameters
  (`f: (Int => Int) = _ + y`), `None` and `Nil`, and an implicit conversion are taken as in a `val`, where teq
  answered `unreachable case` or `missing parameter type`, and the variable has the written type. Programs that
  now run where they threw: a right side that is null (`l: Item = make(y)`) gave `MatchError: null`, and a
  converted one failed the same test. Two new refusals, scalac's: a right side that does not conform to the
  written type (E007; teq warned and built a program that failed) and a use of the variable beyond its written
  type (`l: Any = "st"` then `l.length`, E008). A lowercase name in the alias's type is the type of that name, not
  a pattern's type variable, and the variable is in scope in its own initialiser, where reading it is the forward
  reference scalac tells (E039). Not covered yet: an alias followed by a guard (`..; l: T = e; if c`) is still
  typed as a pattern.
- A forward reference that extends over the definition of a value is refused, as scalac refuses it (E039): in a
  block, a value defined at or before a later method, class, object, given or lazy value that something standing at
  or before the value names (`def first = second; val held = first; def second = 1`), a value named in its own
  initialiser, a later class created, extended or matched by a constructor pattern, a default's getter of a later
  method. The check stands after the inlining phase, as scalac's does: a later inline method called over a value is
  accepted, what its expansion names is checked, an inline method's own forward reference is told once at its
  definition, and a quote's copy at the macro's call. Such programs were built before and failed at run time (`the
  local second is not in scope`, a JavaScript `ReferenceError`) or printed what scalac never compiles. Not covered
  yet: a later *value* named ahead and a self constructor call's forward reference are refused with another message
  (`not found`), as before; a later class named by the type of a plain pattern's selector (`(x: Later) match { case
  y => .. }`), which is a reference to scalac, is still built.
- A typed pattern is a forward reference too where its type is written by a later class, trait, enum or type alias
  of the block, or by `T.type` a later object or lazy value (`case x: Later`, `case _: Later`, `case x @ (_: Later)`,
  `case _: Later.type`), as under scalac (E039, told at the type for a typed variable and at the `_` for a typed
  wildcard): the pattern's wildcard is an identifier of the written type there. An applied type, a union, an
  intersection, a member of a later object and an alias written before the value name nothing, as before.
- A class of a block whose own classes read givens of the block through inline methods takes those captures in
  one order however the program is built: the classes it declares are taken by their place when the captures are
  completed after the inlining phase. A module built alone over the program's own products gave such a class
  another constructor than the whole build did (`class Outer { class Z { .. Lib.b .. }; class Y { .. Lib.c .. } }`:
  `Outer(a, b, c, d)` whole, `Outer(a, c, d, b)` in parts), so the part build's class files and JavaScript were
  not the whole build's. Where a class declares two or more such classes, a whole build's constructor can take
  them in another order than before (their places, where it was their names'); no program's meaning changes.
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
- The errors for the members a class inherits from two unrelated traits (`class C inherits conflicting
  members: …`) come in one order at every worker count: the members of the trait mixed in last in that trait's
  order, then those of the trait before it; for the program's and a library's traits the first error is the one
  scalac reports.
- A method or value whose result type is inferred, a default argument, an inline method and a class's type member
  are typed where they are defined, whichever use asks for them first, as scalac types them. One first used inside
  a `match` case that refines a type parameter, a given's right-hand side, a constructor's call of another
  constructor, the arguments of a `new` or of a parent, a quote, or the body of another method of the same block
  no longer takes that code's bounds, given, constructor or names: `def shown = which(held)` in a `class Keeper[A]`,
  first called under `case IntTag`, picks `which(x: Any)` as it does everywhere else. A block's inline method may be
  called back by a later method of the block that its own body needs (`inline def shown = "got " + later` above
  `def later = if done then "x" else shown`). Programs that were refused, or built with another overload or another
  given than scalac's, build as scalac builds them, and the same at every number of workers.
- A build no longer ends with `0 errors found` and a failing exit where an inline method's expansion reports an
  `inline val` that is no constant and then another error of the same call (an `inline if` or an `inline match` it
  cannot reduce, a `summonInline` that finds no instance): that error is shown, as under scalac. And what the
  expansion of an inline method that is not transparent, or of a macro that is not, reports is shown by a watch
  session and by the editor as a fresh check shows it: not beside a syntax error, and again once the syntax error
  is corrected; so is an error a transparent macro reports.
- A macro whose code uses a `java.util` map or set no longer fails, in rare builds of a session kept open with
  several workers, with `not supported yet in a macro: no builtin for java.util.Any.storeHas (a @js template
  without a Scala body)` at a call that built a moment before: the interpreter named the method it runs before its
  signature had reached the worker that ran it, and kept that name for the worker's later runs.
- A Java class's static member class is a member of the class's companion alone, as scalac enters it: a class
  below `java.nio.file.WatchEvent` and a class with an inner `Kind` means its own `Kind` in every position (a
  self type, a pattern, a subclass, an export; one such program computed with the wrong class), a name declared
  beside it or imported from an instance is not shadowed by the Java one, and `this.Double` below
  `java.awt.Point` is refused. A library's pickle names such a class in a signature as scalac does
  (`java.lang.ProcessBuilder$.Redirect`), so that a module compiled by scalac can use an inline method of a
  teq-built library that calls an overloaded Java method over one.
- An inline method whose result type is inferred may call another inline method of inferred result that is written
  after it or stands in another file (`inline def outer(x: Int) = inner(x) + 1` above
  `inline def inner(x: Int) = x * 3`): the call no longer builds a program that prints `undefined` or `()`. Two
  such methods that call each other, and one that calls itself, are refused as scalac refuses them (`recursive use
  of outer needs an explicit result type`). Two such methods of one block with a value
  or a given defined between them are not covered yet: that call still builds the program it built before.
- `teq fix` says where it is and how it ended, on stderr: a line where the typing of each job begins, named by its
  project and what it covers (`app: typing main and test (1 of 3)`), and one where its round ends (`app: 12 files, 0
  lints, 1 to rewrite in 840 ms`), why the rounds are typed by one worker, once (`teq fix: typing on one worker,
  because the rule OptionGetWarn reads the symbol index`), a second round as what it is, and a last line with the result and the time, a clean run's too (`teq fix:
  3 projects, 12 files, 0 lints, 0 files rewritten in 2.4 s`; a time under a second reads `840 ms`).
  On a terminal the line of a configuration's start is written over itself with the seconds it has stood
  ([teq fix](https://github.com/Carrot-Inc/teq/blob/master/docs/CLI.md)).
- A transparent inline method reads its `inline` parameter's argument where the parameter is read, with a cast
  written on it, as scalac reads it: `transparent inline def pick(inline b: Boolean) = inline if b then 1 else 2`
  given `flag.asInstanceOf[true]` takes its first branch whatever `flag` holds, and given
  `true.asInstanceOf[Boolean]` is refused (a cast to `Boolean` is no constant), where the value under the cast was
  read; so through a transparent method that passes the argument on, a block
  (`{ val x = flag; x.asInstanceOf[true] }`) and an operation over the cast. Its parameter that is not `inline`
  takes no constant from a cast to a singleton type or to a class, and a by-value one none from a call whose
  result type is a singleton: an `inline match` on it takes its other case, an `inline if` is refused. An inline
  method that is not transparent reads its arguments as before, but its by-value parameter no longer reads through
  an ascription over a cast or a cast to a union or an intersection in a build on one worker: it did there, and
  not in a build on several.
- A module of which some sources are built again over its own products no longer stops with `a wait on a cell
  under the loader's lock` where the products name a val of those sources as a path in one of these ways: an
  export's forwarder of it (`export Stock.*`), a parameter of its singleton type (`def take(x: Stock.alias.type)`),
  `this.tag.type` in a transparent method's body. The val's path is read without its signature, which another
  worker may still be computing, and what the val stands for is asked where a type is compared.
- Such a part build answers as the whole module's build where a given class of the products is typed by a val
  of the sources built again (`given aliasShow: Show[Stock.alias.type] with …`, an implicit object, or through a
  type alias of the products) and that val's own initialiser searches for a given: the search sets the candidate
  aside, as the whole build does while the given's class is being completed, and the val is typed. Before, the
  part build could be refused with `recursive value alias needs type`. An alias given of the products there, or
  a given typed by another val that is still being typed, is refused as the whole build refuses it.
- A type parameter whose upper bound is a refinement with a `def` or a `val` (`T <: AnyRef { def self: Int }`), and a
  higher-kinded one applied to arguments (`F[X] <: AnyRef { def self: X }`), conforms to a refinement that asks
  for that member, the bound written on the parameter or reached through another parameter, an intersection or
  an alias: `def keep[T <: AnyRef { def self: Int }](t: T): AnyRef { def self: Int } = t`, and
  `class D[G[X] <: { def self: Int }] extends C[G]` for a `class C[F[X] <: { def self: Int }]`, were refused (a type
  mismatch, "type argument G does not conform to upper bound") and build as scalac builds them.
- A class whose type parameter takes a type constructor and has a bound (`class Later[F[G[_]] <: AnyRef]`,
  `class Later[F[X] <: Iterable[X]]`), a top-level class, an object's or a block's, is accepted where it is
  applied before the class is completed: in a file typed before the class's, in the class's own constructor or
  parents, in a block before the class (it was refused with a kind mismatch); and an argument outside that bound
  is refused there too (the program was built).
- On JavaScript a value class below a universal trait that overrides `equals` or `hashCode` (`class Meters(val n:
  Int) extends AnyVal with Rounded`) compares and hashes by the trait's, as on the JVM and under scalac; it compared
  and hashed by its field. On every target it keeps its own `equals` and `hashCode` where the trait's `equals`
  goes by another `@targetName`, and in a source under `import scala.language.`2.13``, whose classes are Scala 2's
  to the code that uses them, in the same build or from another module, as they are under scalac: a call without
  `()` of their methods warns, their alphanumeric methods are operators, a case class's mirror is not its companion.
- An import through a value's member (`val shelf = new Shelf(..); import shelf.row.*`, `import shelf.row.given`)
  is the value's own in a build typed on several workers, and in the expansions of an inline method whose body
  holds one: two such imports typed or expanded at the same moment could be read as each other's, so that a build
  refused a correct line (`not found: label`, `No given instance of type Badge was found`), or passed and named
  another method's value at run time, or stopped with a crash; the more so in a large build, whose inline calls
  are expanded on several workers. One worker's build was never affected.
- A build on several workers no longer stops with `a wait on a cell under the loader's lock` at a class that
  mixes in a trait nested in a class or in another trait (`class Crate extends rack.Slot`; `Tools.Entry` of an
  `object Tools extends Catalog[Tool]`) while another worker is still typing the trait's abstract methods or
  vars: whether such a trait needs its enclosing instance is read from how its members are written (abstract, no
  default argument), as scalac reads it, and no member's type is asked. A check of one small file could stop one
  time in five on eight workers. And a program that tests for such a trait, or defines a local class of it,
  inside the initialiser of a val that the trait's own methods or vars name (`def fit(a: Yard.held.Label): Int`
  in `Slot`, and `val held = { x match { case _: rack.Slot => … }; new Holder }`) builds as scalac builds it;
  it was refused with `recursive use of fit needs an explicit result type` or `recursive value held needs type`.
- A local enum's `valueOf` and `fromOrdinal` name the enum in their exceptions on the JVM as scalac does
  (`enum demo$package$._$Tone has no case with name: mid`, `... has no case with ordinal: 0`: the enum under the
  object of its file) whatever the number of typing workers; a build typed on more than one worker wrote the bare
  name (`enum Tone has no case ...`).
- A class that joins a method the standard library declares with a trait's method of the same name (`abstract
  class Guard extends PartialFunction[Int, Int] with Tester`, `Tester` declaring another `isDefinedAt`) no longer
  renames the library's method for the whole program: partial functions, sequence patterns, `compareTo`, an
  exception's cause and plain function values work beside it on JavaScript and in the interpreter (`TypeError:
  pf.isDefinedAt$Any is not a function`), and a class implementing the library's method runs on the JVM
  (`NoSuchMethodError`). A method that implements both the library's method and a program trait's joined one
  (`class Both extends Runnable with Own`) builds and answers through both types, where teq refused it ("overrides
  methods that are called run$0 and run").
  A class's value or method that implements a trait's method which another class joins with an overload answers
  through the trait on JavaScript and in the interpreter (`TypeError: c.size$0 is not a function`), also where a
  `super` call of the name comes before it (`TypeError: o.run$0 is not a function`).
- `teq compile` answers by the project asked for and says what the build did: `app: compiled main and test, 31
  classes written, in 107 ms`; `app: compiled main and test, no class changed, in 87 ms` after an edit that changed
  no class; `app: up to date` where nothing was typed again; `web: checked main in 4.8 s` for a Scala.js project;
  `app: 2 errors` after the diagnostics. A project that another project's build served has no count of classes
  (`core: compiled main in 0.1 s`), and errors outside the project asked for are said (`core: 1 error in app`; `app:
  5 errors, 2 in core` where only some are).
  Several projects get a line a build, named by the projects asked for that it covered (`app, core: ..`), those up
  to date none, and a total on stdout, `teq compile: 7 projects in 0.3 s` (`, 1 error`), or the one line `teq
  compile: 7 projects, up to date`. The configuration's key (`app/test`) and `the first build` are no longer
  printed. `teq compile` and `teq run` say that they wait: a build that has not answered a second after the command
  began is named on stderr (`compiling api, app, core (1 of 2)`; on a terminal a line written over itself with the
  seconds, `compiling app, web (5 projects), 14 s`). `teq run` prints its build's line only where the build wrote a
  class or failed: a run with nothing to compile prints the program's output alone. `teq test` and `teq stage` name
  the project too (`app: 4 suites, 5 passed, 0 failed in 423 ms (built in 3 ms)`, `app [fast]: 2 suites`, `app: no
  suite to run`, `teq: app: the compiler for app ended: ..`), and a generator's line its project (`browserdemo:
  images: ..`; `api (test): ..` for one of the test configuration). A time of a second and more reads in seconds
  (`4.8 s`) in every verb's lines. A script reads the records, not the lines (`--diagnostics`): a build's outcome
  and a test run's counts are records now, and a failure's record says what kind it is
  ([teq compile and run](https://github.com/Carrot-Inc/teq/blob/master/docs/TOOLING.md)).
- A command that meets a daemon of another teq binary or of an export that changed no longer ends with `teq: the
  daemon asked for a restart three times`: it starts the next daemon, never asks the ending one again, and says
  that it waits for it (`teq compile: waiting for the previous teq daemon to stop, 3 s` on a terminal, `teq: waiting
  for the previous teq daemon to stop` otherwise). A compile, and the build before a `teq run`, whose daemon ended
  before it answered anything is sent to the next daemon; `teq test` and `teq stage` still stop there (`teq: the
  daemon ended without an answer`), since their work may have begun, and so does any command whose daemon said
  nothing and is still running. A command no daemon served leaves a `--diagnostics` file with that failure's record
  alone, not the records of an earlier run. The file holds one command's records, whole: it is empty while the
  command runs and is written when it ends, so two commands that name one file no longer leave both their records
  in it.
- A build on several workers no longer stops with `a wait on a cell under the loader's lock` where a method is
  called on an intersection whose two parts overload its name (`x.at("k")` for `x: Reads & Writes`, each trait
  with several `at`) while another worker is still typing what those overloads' signatures name: the overloads
  are compared first, where the other worker is waited for, and the member they make is then recorded for every
  worker. A build of three small files could stop four times in five on eight workers. Where such overloads'
  signatures depend on each other in a cycle across files, the build is refused with one worker's error at
  every count (`recursive use of at needs an explicit result type`), where it stopped.
- A class declared inside a method's body with a val beside an inherited method of the same name (`class Pan
  extends Scale { val weight = 5 }` where `Scale` has `def weight(step: Int)`) no longer stops the build with
  `a named method has its name`: on several workers where a macro's implementation runs that method, and on any
  number of workers where the class is declared in an inline method.
- A call that passes an explicit argument to a context bound's evidence (`stamps.one(Seal[Int]())` for
  `def one[X: Seal]`) is reported under `-source 3.4` and `3.5` (`Context bounds will map to context parameters`)
  whatever the number of typing workers; a build typed on more than one worker left the record out for a method
  whose signature another worker had completed. The same call through an export's forwarder (`export stamps.one`,
  then `Desk.one(Seal[Int]())`) is reported too, as scalac reports it: it was not, on any number of workers, so a
  program that passed such an argument under `-source 3.5`, or to a library compiled under it, is now refused.
- A macro's implementation that runs a method whose body declares a class (`def total(n: Int) = { class Tray(..)
  extends Drawer[Int] { .. }; .. }`, called from the macro) gives the same result on several typing workers as on
  one. Where the class implemented both alternatives of a generic trait's overloaded method (`def take(k: A)` and
  `def take(k: String)` for `Drawer[Int]`), a build on more than one worker printed other numbers, with no error;
  where the class had a lazy val, an object as a member or a class declared in one of its methods, or its members
  read a val or a var of the method's body, the build stopped (`exception in macro`, `not found: seen`).
- An argument that an inline method takes in the place of its parameter (a field of the calling class, `this`, a
  by-name or an inline parameter's code, a lambda) is the caller's where a class declared in the method's body reads
  the parameter: `inline def over(k: Int) = { class Two { def put = k }; new Two().put }` called as `over(this.n)`
  inside a class read `n` from the new `Two`, so JavaScript printed `undefined` or `0` without an error, the
  interpreter `0`, and the JVM threw a `ClassCastException`; with the parameter passed to the class's parent
  (`class Two extends Base(k)`) JavaScript stopped with `Must call super constructor`. A named or an anonymous
  class, a lambda for a trait, an object and a given of the body are covered, and a method that passes the
  parameter on to another inline method. Not yet: the parameter as a constructor's default of such a class
  (`class Two(val got: Int = k)`), and a class declared inside another class of the body on the JVM. The same
  holds for a value a block imports from a field of the enclosing instance (`import b.m` in a method of a class
  with the field `b`) and reads in a class, an object or a lambda for a trait of that block: such a read stopped
  the program (a `TypeError`, a `NullPointerException`, a `ClassCastException`).
- A build on several workers no longer stops with `a wait on a cell under the loader's lock` where a library's
  data names a definition another worker is typing, as the products of a module do when some of its sources are
  built again over them (`class Sub extends Stock.api.Request` in the products, `Stock` among the sources built
  again): the build is typed again by one worker, with one worker's output, and says so in one line where the
  number of workers was automatic.
- A class declared in an inline method's body is a whole class at each call. Its methods that implement a generic
  parent's are reached through the parent on the JVM too (`class Two extends Op[Int] { def put(x: String) ..; def
  put(x: Int) .. }` called as an `Op[Int]` threw `AbstractMethodError`); a method whose parameter or result is the
  class itself, a class nested in it, a sibling of the body or an array of them, and a type parameter bounded by one
  (`def pick[T <: Two]`, `class Box[T <: A0]`), no longer stop the JVM with `NoClassDefFoundError`. Where a class or
  trait of the body declares two methods of one name, a class of the body that overrides or implements them is called
  as scalac calls it: `t.put(1) + t.put("abc")` on such a class printed another number on JavaScript without an
  error, and the interpreter and the JVM stopped. The same holds where the two methods come from two unrelated
  traits and the class declares neither (`class Two extends M1 with M2`): the JVM stopped with `VerifyError`, and
  with the traits declared in the body JavaScript called one of them for both. Not yet: a signature that names a
  class of the body through another type (`A#In`, `a.In`, `Array[? <: A0]`), and a call whose inferred type argument
  is such a class, still stop the JVM; over another module's products JavaScript and the interpreter still call both
  of such a class's own two methods by one name, and a class with the two unrelated traits is refused there or
  prints another number; and a call whose type arguments make the body's class invalid (its method then overriding
  an inherited one without `override`) is still built, where scalac refuses the call.
- A method that the override check refuses is reported once: `def put(a: String): Int` in a class extending
  `Ops[String]`, whose `put(x: A): Int` and `put(x: String): String` it both overrides, got a second error about its
  name in the JavaScript output after the first.
- A macro's implementation that runs code which calls the forwarders of an `export` (`class Switch(c: Coil) {
  export c.{pull, mark} }` with `pull` overloaded, an object's `export base.*`, a trait's `export inner.pull as
  tug`) or creates a class through a secondary constructor (`new Crate("abc")` for `def this(s: String) = ..`,
  with or without a default argument) gives the same result on several typing workers as on one. On more than one
  worker a call of an overloaded forwarder could run another alternative, so the program printed other numbers
  with no error, and a secondary constructor stopped the build (`value apply is not a member of Crate`).
  Not yet: an `export` in a trait that stands in a file of its own and is mixed into a class of another file
  (`trait Geared { export drum.wind }`): a call of an overloaded forwarder there can still run another
  alternative.
- A module that calls another module's `transparent inline` method whose body declares a class (`new Rod(n) {
  override def label = .. }`, a typeclass instance made in the method) writes the same pickles whatever the number
  of typing workers. With two workers or more the class could be written without its members, and what reads the
  caller's body from the pickle took it so: a later module's build for JavaScript, the interpreter, and a later
  module's macro that runs the caller, on the JVM too. Where the class overrides a member the program printed the
  parent's value with no error (`rod abc` for `got abc`); where it implements an abstract one it stopped (`b.label is
  not a function`). The module's own class files were right. A module of more than 512 KiB of sources is typed by
  several workers without `--threads`, so this did not need the flag. With it:
  a class declared inside such a class (`new Rod(n) { class Notch(val at: Int); .. }`) no longer stops the calling
  module's build, as it did at one worker (`references to definitions never written`): the module builds and the
  caller's body alone is left out of its pickles; and a member of such a class declared by the singleton type of a
  member of the class itself (`val got: this.len.type`) has that member's type in the class file an expansion of a
  plain `inline` method makes (`int got()`, where teq wrote `Object got()`).
  Not yet: where the pickle cannot hold a member of such a class yet, the caller's body is left out of the module's
  pickles (`the body of .. is withheld from the products`, with the reason), now at every number of workers; two
  workers or more used to write the class without its members, which a later build accepted when it needed none of
  them. The members that can do this: a class, a trait, an object or a given with a body declared in the class;
  one declared by the singleton type of a member of the class itself (`val got: this.len.type`); one whose body
  declares a class (`def spare(k: Int) = { class Notch(val at: Int); .. }`); and a type member that names the
  method's type parameter (`type Elem = T`).
- `teq fix` types the projects of a run and runs their rules side by side, as many at once as the machine has
  workers (`--threads n` bounds them): each project in a process of its own, and what the run prints
  and writes is unchanged, line for line and byte for byte. `--jobs n` runs at most `n` at once, `--jobs 1` one
  after another as before. On a terminal the line that shows where the run is names the projects done, those being
  typed and those whose rules run (`teq fix: 2 of 17 projects done, typing api, core, web, 14 s`).
- `object Main extends App` builds with teq's own std, for JavaScript and in the interpreter: the std declares
  `scala.App` and `scala.DelayedInit` as scala-library does. As in Scala 3, the object's body is its initialiser
  and `main` is the one it inherits, so `args` is `null` while the body runs; a block the program itself gives
  `delayedInit` is run by `main`. Before, such a program was refused (`type App not found`) unless it was built
  for the JVM or with `--std=scala-library`, which read scala-library's own `App`.
  A build with teq's std that has scala-library's jar on its class path used to take `App` from the jar; it takes
  the std's now, as for every name the std defines. Its output is smaller, and a block that gives `delayedInit`
  another block while `main` runs them stops with scala-library's `ConcurrentModificationException`, where the
  jar's code over teq's `ListBuffer` went on.
- A deprecation warning for a name of teq's own std is given wherever the name is written, whatever the build
  named before it and at every `--threads` count: a type (`def size(s: Stream[Int])`), a parent
  (`object Main extends App`), a member (`xs.toStream`) and a value are each checked by their own definition's
  mark. Before, a type, a parent or a member of the std was warned of only after a deprecated value or a
  `@deprecated` definition of the program's own had been met: `def size(s: Stream[Int])` alone built without
  scalac's warning and passed `--werror`, and with several files the number of warnings turned on which file
  named the value and on the worker count (three files: scalac's 3 at one worker, 1 or 2 at more).
  The std's marks are scala-library's in text and origin: the class and the object `Stream` carry its messages
  (a build that silences one by `--wconf msg=...` no longer fails with the jar on its class path), a warning
  for `toStream` names `IterableOnceOps`, where scala-library declares it, in its text and in its origin
  (`--wconf origin=scala.collection.IterableOnceOps.toStream:s`), an origin of a definition directly in
  package `scala` is spelled as scalac spells it (`App`, `Stream`, `App.delayedInit`), and `$conforms`,
  which scala-library does not mark, is no longer marked.
  A warning names a definition of a nested package with the package's full name, as scalac does (`class Stream
  in package scala.collection.immutable`), and its origin leaves out `scala`, `scala.Predef` and `java.lang`
  where scalac's does (`StringFormat.formatted`).
  Not yet, each silent where scalac warns: a deprecated definition reached through an export's qualifier or
  forwarder, an import's qualifier (`import scala.Stream.*`), an extractor pattern (`case Stream(a, _*)`), a
  member whose result is applied at once (`xs.toStream(0)`), a given the search selects, an implicit argument
  or conversion the compiler inserts, an extension method, a `derives` clause, a `for`'s `withFilter`, an
  interpolator, a member type through a dependent path, a type argument the compiler infers
  (`List(List(1)).map(_.toStream)`, where scalac warns of the class `Stream` too), a singleton type
  (`Stream.type`), or a default argument of a method that is deprecated itself; and the definitions
  scala-library marks that the std does not yet (`toIterator`, `filterKeys`, `replaceAllLiterally` and others).
- A brace block after a symbolic name selected with a dot belongs to that name, as scalac reads it: the method's
  argument (`xs.++ { .. }`, `xs.:: { x }`), a type's refinement (`val v: O.+ { def f: Int }`), an instance's body
  (`new O.+ { .. }`), a package's body (`package outer.+ { .. }`). teq read the brace as an infix operator's right
  operand and refused the line (`expected ',' or ')', found '{'`, `expected end of statement, found '{'`). An
  operator between two operands keeps its brace as its right operand (`xs ++ { .. }`).
- An extension's methods are read from the indented lines after its header wherever the header's last clause
  closes, as scalac reads them: a header written over several lines (`extension (`, the receiver on a line of its
  own, `)` on the next, then the methods), a clause broken inside its brackets, a blank line or a comment between
  the header and the methods, the methods' brace on the line after the header. teq read the methods only where
  the header's last line began with the word `extension` and refused the rest (`expected extension methods`).
  In backquotes the word is a name: `` `extension`(1)(2) `` at a statement's start calls a method of that name.
  Where an extension is written on the line of an opening brace and its header breaks inside its parentheses
  (`object A { extension (x: Int)(using`, then `y: Int) def ..`), the brace's later lines take their width as
  scalac gives it: a line teq read as a member of its own can now belong to the statement before it.
- A line that begins with a `.` after an indented block continues that block, as scalac reads it: `def f =`, an
  indented `for .. yield ..`, then `.value` at the def's column selects on the whole right side; at an `if`'s
  column it selects on the else branch alone. The same holds for a `match` and, inside parentheses, a type
  application on such a line, and for the block that is a case's body (`case 1 =>`, two indented lines,
  `.reverse` in a match written in braces). teq refused these lines (`expected end of statement, found '.'`,
  `expected 'case', found '.'`); after the indented else branch of an `inline if` it read a `match` at the `if`'s
  column on the whole `if`, where scalac matches the else branch; and with cases written at their `match`'s own
  width, the last one's body a block in braces on its line, it read a `match` further in on the whole match,
  where scalac matches that block. An argument list on such a line still begins a statement. Where a `match`
  stands in what continues an indented block or the block of a case's body, its cases at its own width and a
  line further in than the cases after them, the program is refused at the token that continues the block
  (`expected end of statement, found 'match'`): scalac reads that line as the last case's second statement,
  which teq does not yet. Two shapes of it teq read before, by another grouping and with the line as a statement
  after the match: after the else branch of an `inline if` (the `match` on the whole `if`), and after a case's
  block in braces with the `match` on the case's line (the `match` on the whole first match). Such a program
  printed what scalac prints where the grouping did not show (one case, the else branch the one that runs); it
  is refused now. Not yet as scalac: after a case's block that a selection or an operator continues, a `.` or a
  `match` on a later line of the cases' width is read on into the continuation, where scalac ends the cases at
  that line; teq reads such a line the same way after a case's body that is no block.
- An indented block of one expression is that expression, as scalac's parser makes it; a block in braces stays a
  block. A selection on the line that closes the indented block chooses among the overloads of the expression's
  method by the member selected, and `for x <- xs yield` with `x` alone on the indented line after it yields `xs`
  itself, as `for x <- xs yield x` does (teq read the indented `x` as a block's expression and mapped the
  collection).
- A line that begins with `(`, `[` or `{` and stands further in than the block it is in goes on with the line
  before it, as scalac reads it: it is the argument list, the type argument list or the block argument of what
  that line ends in, with a comment line between them or not. After a blank line (one inside a block comment
  too), at the block's own indentation, and under ``import scala.language.`3.0-migration` ``, such a line begins
  a statement.
  teq decided this from what the line before ended in, and read some of these programs otherwise than scalac or
  refused them; braces whose first line break falls inside parentheses now take the indentation of the block
  around them, so a line of theirs that begins with a parenthesis continues the line before it too.
- A `for` whose enumerators begin on its own line, without parentheses or braces, goes on over the lines after
  it, at its own indentation or further in, up to its `yield` or `do` (`for x <- xs` / `y <- ys` / `yield ..`).
  teq refused the second line (`` `yield` or `do` expected ``).
- The cases of a `match` or a `catch` and their bodies are divided as scalac divides them. A case's body is every
  line up to the next `case`: lines further in than the cases after a body that began on the case's line, or after
  a body in braces, belong to it, and under `{ case x =>` a later line left of the body's first line does too
  (teq refused these: `expected 'case'`, `expected end of statement`). Cases indented below their
  keyword, or written at its own indentation, end at the first line of their indentation that does not begin with
  `case`; that line is the next statement, or continues the whole `match` when it begins with `.` or an operator.
  A `case` on the same line as its `match`, without braces (`x match case 1 => ..`), is refused as scalac
  refuses it (`expected '{', found 'case'`); teq accepted it.
- Indentation that mixes tabs and spaces is measured and compared as scalac does: a line's indentation is its first
  run of one character and the run after it, which is all scalac keeps of it (a line led by a space, a tab and two
  more spaces is as wide as one led by a space and a tab), and a line belongs to a block whose indentation it
  extends. Where neither indentation extends the other the line is refused with scalac's error (`Incompatible
  combinations of tabs and spaces in indentation prefixes`, with the two indentations); teq compared the counts of
  characters and could put such a line in the wrong block without a word. A line that matches no enclosing
  indentation is reported with its widths in scalac's words (`1 tab, 2 spaces`). A source file's outermost block
  is as wide as the line its first token stands on (after a script header, the line after the header), and a
  line left of it narrows the block, as in scalac; teq's was 0 wide. A Scala CLI script's statements are read in
  its wrapper class, from the start of a line, whatever its first line's indentation, as before.
- A type argument list after an expression written in braces applies to the value of the block, as in scalac:
  `{ poly }[Int]` calls the value's `apply`. teq handed the type arguments to a method the block named, and where
  that method's result had an `apply` with type parameters it gave the method's result itself, a wrong value. A
  method named in braces that has no such `apply` is refused (`{ id }[Int](1)`).
- After `return`, a block in braces followed by an argument list is refused as scalac refuses it (`expected end
  of statement, found '('`); what continues a block (a selection, an operator, a `match`) continues the operand.
- A type projection goes on over a line break after its `#` or before it (`A#` / `B`); teq ended the statement
  at the line break and refused the next line.
- After `if (c)`, `while (c)` or `for (..)`, a line further in that begins with a bracket is the branch or the
  body itself and opens no block, as in scalac: a second statement on the line after it is not part of the branch,
  and an `else` below it is refused (`expected end of statement, found 'else'`). teq read both lines as the
  branch's block.
- A view or an extension method applied to a receiver written as a path or a literal (bare or in parentheses) takes
  the receiver's own type, as scalac does, where the member lookup widened it: a literal, a singleton or a union of
  constants keeps its own type for the method's type variable, which is widened only where nothing bounds it; and a
  `val`, a `var`, a `lazy val`, a parameterless `def` or a by-name parameter whose type is an alias of a union keeps
  the alias. A receiver written any other way (an ascription, an annotation, a block, an `if`, an application) is
  taken as before. So
  `val o: Option[Stage] = "final".some` (an `implicit def` view's `.some`, `type Stage = "draft" | "final"`),
  `val p: (Stage, Int) = "final" -> 1` and `stageVar.some` for an `Option[Stage]` compile, which teq refused
  ("found Option[String], required Option[Stage]"); an extension whose given its type variable chooses finds
  `Label[Stage]` for a receiver of `Stage` where it found `Label[String]`; and one whose context bound the
  alias does not meet is refused, as scalac refuses it.
- A wildcard argument of a written type application is held to its parameter's bounds by its two ends, as scalac holds
  it: `type A = Up[? >: String]` for `class Up[T <: Int]` is refused ("type argument ? >: String does not overlap with
  upper bound Int"), and `Up[? <: Int]` built; teq skipped every wildcard argument. A bound that names another
  parameter whose argument is a wildcard meets it as the reference's `<app>#A` (`class C[A, B <: Set[A]]`, `C[? <:
  Int, Set[Int]]` is refused against `Set[C[? <: Int, Set[Int]]#A]`), an F-bounded argument is narrowed where scalac
  narrows it (`def f[S <: BS[?, S]](x: BS[?, S])` builds; through an alias it is refused), and an abstract type
  member's or a higher-kinded type parameter's application is checked too (`t.M[String]` for `type M[X <: U]` of `t:
  T[Int]`). A nested class's bounds are seen from its prefix (`new o.In[Int](5)` for `class Outer[X]: class In[T <:
  X]` and `o: Outer[Int]` builds, which teq refused), a bound that is a kind alone holds no argument back (pos/X
  builds), a bound or an argument reaching `caps.Pure` is held as before, a check in a `case` body sees the case's
  GADT constraints, and nothing is checked inside a match type's cases. As scalac, a bounds error is not reported
  after the typer's errors, and no error of a later phase is reported after it.

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
