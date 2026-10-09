object T:
  class Box[T <: Int](t: T)
  class OnlyInt[T <: Int]
  type L = List[OnlyInt[String]]
  def baz[X, Y >: X](x: X, y: Y) = y
  val b: Box[String] = ???
  def test = baz[Int, String](1, "abc")
  def up[T <: Int](t: T) = t
  val c = up[String]("s")
  trait Ord[A]
  def sorted[A: Ord](xs: List[A]) = xs
  given Ord[Int] with {}
@main def run(): Unit = println(1)
