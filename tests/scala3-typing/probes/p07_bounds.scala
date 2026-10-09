object T:
  def g[T <: Int](t: T) = t
  class Box[T <: Int](t: T)
  class OnlyInt[T <: Int]
  type L = List[OnlyInt[String]]
  def baz[X, Y >: X](x: X, y: Y) = y
  def test =
    g("foo")
    val b: Box[String] = ???
    baz[Int, String](1, "abc")
@main def run(): Unit = println(1)
