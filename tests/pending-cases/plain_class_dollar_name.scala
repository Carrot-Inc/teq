// A plain class whose name ends in `$` is shown without it where dotty shows a name, pinned to M10.
// Two members of dotty decide it:
// - a case class's `productPrefix` (and so its `toString`) is `SyntheticMembers.ownName`
//   (SyntheticMembers.scala:106), `clazz.name.stripModuleClassSuffix`:
//   scalac prints `Foo(1)` and `Foo`;
// - a diagnostic shows a name through the printer's `nameString` (RefinedPrinter.scala:88-92), which strips it too:
//   for `class Box$; val x: Int = new Box$` scalac reports `Found: Box` (a diagnostic, so no line
//   of this program's output).
// `stripModuleClassSuffix` (NameOps.scala:155-160) unmangles a simple name ending in `$`, not only a module class's.
case class Foo$(x: Int)
class Box$
@main def run(): Unit =
  println(Foo$(1))
  println(Foo$(1).productPrefix)
  val b: Box$ = new Box$
  println(compiletime.codeOf(b))
