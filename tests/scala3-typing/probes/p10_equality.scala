class A
class B
enum Color:
  case Red, Green
case class P(x: Int)
@main def run(): Unit =
  println((new A) == (new B))
  println(1 == "s")
  println(List(1) == Nil)
  println(Some(1) == None)
  println(Color.Red == "Red")
  println(P(1) == "x")
  println(Option(1) == Some(1))
  println(1 == 1.0)
  println(Color.Red == Color.Green)
  println(P(1) == P(2))
