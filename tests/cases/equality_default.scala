// Comparisons scalac accepts without strictEquality: related types, the standard instances,
// and pairs where neither side has a reflexive CanEqual.
class A
class B
enum Color:
  case Red, Green
enum Shape:
  case Circle
case class P(x: Int)
def gen[T](xs: List[T], ys: List[String]) = xs == ys
def gen2[T](x: T, s: String) = x == s

@main def run(): Unit =
  val a: Any = 1
  val s: Option[String] = Some("a")
  val e1: Either[Int, Nothing] = Left(1)
  println(Color.Red == Shape.Circle)
  println(Some(1) == None)
  println(Some(1) == s)
  println(List(1) == Vector(1))
  println(List(1) == List("a"))
  println(Set(1) == Set("a"))
  println(a == 1)
  println(1 == 'a')
  println((new A) == (new A))
  println((new A) == (new B))
  println(Left(1) == Right("a"))
  println(e1 == Right(2))
  println(P(1) == (new A))
  println(Color.Red == P(1))
  println((1, P(1)) == (1, new A))
  println(Map(1 -> P(1)) == Map(1 -> new A))
  println(gen(List(1), List("a")))
  println(gen2(1, "a"))
  println(1L == 1.0)
  println(() == ())
