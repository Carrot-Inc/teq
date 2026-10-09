// An annotation's argument whose typing completes a method of another object, whose result the
// method's body gives: the body is typed outside the annotation, its constructor's named arguments
// hoisted in source order as anywhere (`side(2)` before `side(1)`), not as an annotation's are.
class Pair(val a: Int, val b: Int)
class Ann(x: Any) extends scala.annotation.StaticAnnotation

object C:
  @Ann(B.value) def f = 0

object B:
  def side(n: Int): Int = { print(n); n }
  def value = new Pair(b = side(2), a = side(1))

@main def run(): Unit =
  B.value
  println()
