// expect: type argument String does not conform to upper bound Int
// expect: type argument String does not conform to lower bound Int
object T:
  class Box[T <: Int](t: T)
  class OnlyInt[T <: Int]
  type L = List[OnlyInt[String]]
  def baz[X, Y >: X](x: X, y: Y) = y
  def up[T <: Int](t: T) = t
  val b: Box[String] = ???
  def t = baz[Int, String](1, "abc")
  val c = up[String]("s")

@main def run(): Unit = println(1)
