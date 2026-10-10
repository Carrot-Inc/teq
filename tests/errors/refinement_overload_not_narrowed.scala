// expect: 9:21: error: Refinements cannot introduce overloaded definitions
// expect: 12:19: error: type mismatch: found Any, required String
// A `def` refinement is the denotation of the member it describes only: a call of another
// overload of the name keeps its own result. scalac rejects the call too, and before it the
// refinement itself (`Typer.typedRefinedTypeTree`).
class Base:
  def m(x: Int): Any = x
  def m(x: String): Any = x
type R = Base { def m(x: Int): String }
@main def Main(): Unit =
  val r: R = (new Base).asInstanceOf[R]
  val s: String = r.m("a")
  println(s)
