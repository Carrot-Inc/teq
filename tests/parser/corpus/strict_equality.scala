//> using options -language:strictEquality
// teq: --strict-equality
// Every comparison here has a CanEqual instance: predefined, standard, derived or given.
enum Color derives CanEqual:
  case Red, Green

enum Shape derives CanEqual:
  case Circle(r: Int)
  case Square(side: Int)

case class P(x: Int) derives CanEqual
class A(val n: Int)
class B(val n: Int)
given CanEqual[A, B] = CanEqual.derived

def same[T](a: T, b: T)(using CanEqual[T, T]): Boolean = a == b

@main def run(): Unit =
  val shape: Shape = Shape.Circle(1)
  println(Color.Red == Color.Green)
  println(Color.Red != Color.Green)
  println(shape == Shape.Circle(1))
  println(Shape.Square(2) == shape)
  println(P(1) == P(1))
  println(A(1) == B(1))
  println(1 == 1L)
  println(2.0 == 2)
  println('a' == 97)
  println("a" == "a")
  println(true == false)
  println(Some(1) == None)
  println(Option(P(1)) == Some(P(1)))
  println(List(1, 2) == Vector(1, 2))
  println(Set(1) == Set(1, 1))
  println(Nil == List(1))
  println((1, "a") == (1, "a"))
  println((1, P(1)) == (1, P(2)))
  println(Map(1 -> "a") == Map(1 -> "a"))
  println(Left(1) == Right("a"))
  println((Right(1): Either[String, Int]) == Right(1))
  println(same(1, 2))
  println(same(P(1), P(1)))
  println(() == ())
