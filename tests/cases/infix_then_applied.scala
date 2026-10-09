// `(a op b)(args)` is `a.op(b)(args)`: the further list fills the operator's using clause when
// it has one, and applies the result otherwise.
final class Same[A](val test: (A, A) => Boolean)

extension [A](self: A)
  def ~=~(other: A)(using s: Same[A]): Boolean = s.test(self, other)

@main def run(): Unit =
  val loose = Same[Int]((a, b) => (a - b).abs <= 1)
  given Same[Int] = Same[Int](_ == _)
  println((1 ~=~ 2)(using loose))
  println(1 ~=~ 2)
  val inc: Int => Int = _ + 1
  val twice: Int => Int = _ * 2
  println((inc andThen twice)(5))
  println(("ab" + "cd")(2))
  println((List(1, 2) ++ List(3))(2))
