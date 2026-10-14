// A partial application whose clauses left out include a using clause: dotty's `normalize` skips an implicit
// method's clause for its result (`normalize(resultTypeApprox(mt))`), so the function type eta-expansion makes
// of the plain clauses (`A => String`) meets the expected `Int => String` before the lambda is typed, and `x`
// is an `Int`.
def make[A](show: A => String)(a: A)(using DummyImplicit): String = show(a)
def wrap[A, B](f: A => B)(using DummyImplicit)(a: A)(using Ordering[Int]): List[B] = List(f(a))

val f: Int => String = make(x => (x + 1).toString)
val g: Int => List[Int] = wrap(x => x * 2)

@main def main(): Unit =
  println(f(1))
  println(g(21))
