// A positional argument after a named one fills the parameter after the named one's; `copy` of
// a generic case class takes the type parameters afresh, so an argument may change them.

final case class Item(title: String, value: Int, weight: Long = 1L, tag: String = "-")

def make(a: Int, b: Int = 2, c: Int = 3, d: Int = 4): String = s"$a $b $c $d"

final case class Box[T](value: Either[Int, T], n: Int)
final case class Pair[A, B](a: A, b: B, f: A => B)
final case class Sized[T](items: List[T], size: Int)

@main def main(): Unit =
  println(Item(title = "t", 3))
  println(Item(title = "t", 3, 25L))
  println(Item("t", 1, weight = 5L, "x"))
  println(Item(value = 1, title = "u", 35L, "y"))
  println(make(1, b = 20, 30, 40))
  println(make(a = 10, 30))
  println(make(1, d = 40))
  val b: Box[Nothing] = Box(Left(1), 2)
  println(b.copy(value = Right("x")))
  println(b.copy[String](value = Right("s")))
  val e: Box[Double] = b.copy(value = Right(1.5))
  println(e)
  println(b.copy(n = 5))
  val bi: Box[Int] = Box(Right(1), 1)
  println(bi.copy(value = Left(0)))
  val p = Pair(1, "a", _.toString)
  println(p.copy(b = true, f = _ > 0).f(2))
  println(p.copy(a = 3).f(4))
  val s: Sized[Nothing] = Sized(Nil, 0)
  val t: Sized[Int] = s.copy(items = List(1, 2), size = 2)
  println(t)
  println(t.copy(items = t.items.map(_.toString)))
