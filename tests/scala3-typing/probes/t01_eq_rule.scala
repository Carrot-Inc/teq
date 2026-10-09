class A
class B
trait Tr
case class P(x: Int)
enum Color:
  case Red, Green
enum Shape:
  case Circle
@main def run(): Unit =
  println((new A) == "s")          // line 10
  println(P(1) == new A)           // 11
  println(Color.Red == Shape.Circle) // 12
  println((new A: Tr) == "s")      // 13
  println(Some(1) == "s")          // 14
  println(List(1) == "s")          // 15
  println((1: Any) == "s")         // 16
  println(Option(1) == None)       // 17
  println(Color.Red == (Color.Green: Any)) // 18
  println("a" == 'a')              // 19
