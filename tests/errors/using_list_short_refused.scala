// A written `using` list shorter than its clause: an omitted parameter that the search finds no given for is told as the
// search tells it, at the application (E172, without the parameter and the method); a positional argument takes
// the next slot, and the search after a failed argument tells nothing more; a plain list on an implicit clause supplies
// nothing (E171, at the application); a default taken where a given exists is told by E220; the search's words are
// the type's, not a parameter's `@implicitNotFound`; an error elsewhere hides nothing.
// expect: 21:16: error: missing argument for parameter b
// expect: 25:16: error: No given instance of type B was found
// expect: 30:24: error: Found:    B
// absent: 30:16:
// expect: 35:16: warning: Argument for implicit parameter b was supplied using a default argument.
// expect: 40:16: error: No given instance of type B was found
// expect: 44:11: error: No given instance of type B was found
// expect: 45:30: error: Found:    "s"
class A
class B
class C
// The plain list's slot left empty:
// the method's signature after the parameter's name is scalac's words alone.
object Plain {
  def f(x: Int)(implicit a: A, b: B): Int = x
  val result = f(1)(A())
}
object Missing {
  def f(x: Int)(using a: A, b: B): Int = x
  val result = f(1)(using A())
}
object FirstPositional {
  given A = A()
  def f(using a: A, b: B): String = "ok"
  val result = f(using B())
}
object DefaultBesideAGiven {
  given B = B()
  def f(x: Int)(using a: A, b: B = B()): Int = x
  val result = f(1)(using A())
}
object ParamNotFound {
  import scala.annotation.implicitNotFound
  def f(x: Int)(using a: A, @implicitNotFound("custom B missing") b: B): Int = x
  val result = f(1)(using A())
}
object ErrorElsewhere {
  def f(x: Int)(using a: A, b: B): Int = x
  val v = f(1)(using later)
  val later = { val q: Int = "s"; A() }
}
