// expect: value +: is not a member of Int
// expect: type mismatch: found Int, required String
class Vec(val xs: List[Int])
extension (x: Int)
  def +:(v: Vec): Vec = Vec(x :: v.xs)
trait Show[A] { def n: String }
given Show[Int] with { def n = "int" }
extension [A](a: A)
  def *:(prefix: String)(using s: Show[A]): String = prefix + ":" + s.n

@main def run(): Unit =
  val v = Vec(List(1, 2))
  println(v +: 0)
  println("p" *: 1)
