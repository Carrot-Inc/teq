// Secondary constructors that overload beside a primary of a `using` clause alone: a following list's narrowing does
// not choose between them by the written `using` list where the call writes the primary's trailing `()` (scalac
// refuses the call, E050, the `()` being that clause's); teq keeps its own refusal, by `new`, a parent call and an
// anonymous class alike.
// expect: 15:17: error: Ambiguous overload.
// expect: 18:13: error: Ambiguous overload.
// expect: 20:14: error: Ambiguous overload.
class A(val v: Int)
class B(val v: Int)
given A = A(1)
class K(using a: A):
  var value = a.v
  def this(i: Int)(using a: A) = { this(using a); value = 10 + i }
  def this(i: Int)(using b: B) = { this(using A(b.v)); value = 20 + i }
class S extends K()(1)(using B(2))
object Main:
  def main(args: Array[String]): Unit =
    println(new K()(1)(using B(2)).value)
    println((new S).value)
    println((new K()(1)(using B(2)) {}).value)
