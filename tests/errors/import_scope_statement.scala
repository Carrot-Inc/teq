// A top-level import is in scope from its statement on (`Typer.typedStats` over the package definition, the
// imports among the classes and objects in source order): a definition written before it does not see it.
// expect: error: not found: x
// expect: 1 error found
object O:
  val x = 1
object P:
  val y = x
import O.*
@main def run(): Unit = println(P.y)
