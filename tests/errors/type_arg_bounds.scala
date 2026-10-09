// expect: 10:25: error: type argument String does not conform to upper bound Int
// expect: 14:14: error: type argument String does not conform to upper bound Int
// expect: 15:11: error: type argument String does not conform to lower bound Int
// expect: 16:11: error: type argument String does not conform to upper bound Int
// expect: 17:16: error: type argument Int does not conform to upper bound String
// expect: 5 errors found
object T:
  class Box[T <: Int](t: T)
  class OnlyInt[T <: Int]
  type L = List[OnlyInt[String]]
  type Alias[A <: String] = List[A]
  def baz[X, Y >: X](x: X, y: Y) = y
  def up[T <: Int](t: T) = t
  val b: Box[String] = ???
  def t = baz[Int, String](1, "abc")
  val c = up[String]("s")
  val d: Alias[Int] = ???
  trait Foo[F <: Foo[F]]
  class Bar extends Foo[Bar]
  class Ok[T <: Int](t: T)
  val e: Ok[Int] = Ok(1)

@main def run(): Unit = println(1)
