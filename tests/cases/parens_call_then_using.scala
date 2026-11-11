// `(f(a))(using c)` is `f(a)(using c)`: parentheses around the function of an application change
// nothing, so the further list fills the method's own using clause, or applies its result.
final case class Scale(by: Int)

object Fns:
  def scaled(a: Int)(using s: Scale): Int = a * s.by
  def curried(a: Int)(b: Int): Int = a * 10 + b
  def poly[A](a: A)(b: A): List[A] = List(a, b)

final class Box(val n: Int):
  def +>(k: Int)(using s: Scale): Int = (n + k) * s.by
  def apply(k: Int): Int = n + k

@main def run(): Unit =
  import Fns.*
  given Scale = Scale(2)
  val three = Scale(3)
  val box = Box(4)
  println((scaled(5))(using three))
  println((scaled(5)))
  println(((scaled(5)))(using three))
  println((box +> 1)(using three))
  println(((box) +> (1))(using (three)))
  println((curried(1))(2))
  println((poly[Int](1))(2))
  println((scaled)(5)(using three))
  println((box)(3))
  println((new Box(1))(2))
  println(((1, 2))(0))
