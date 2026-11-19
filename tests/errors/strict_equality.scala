// teq: --strict-equality
// expect: Values of types Any and Int cannot be compared with == or !=
// expect: Values of types P and P cannot be compared with == or !=
// expect: Values of types Color and Color cannot be compared with == or !=
// expect: Values of types Some[P] and Some[P] cannot be compared with == or !=
// expect: Values of types T and T cannot be compared with == or !=
// expect: Values of types Email and Email cannot be compared with == or !=
// expect: Values of types Der and Color cannot be compared with == or !=
enum Color:
  case Red, Green
enum Der derives CanEqual:
  case X
case class P(x: Int)
object Ids:
  opaque type Email = String
  def apply(s: String): Email = s
def same[T](a: T, b: T): Boolean = a == b

@main def run(): Unit =
  val a: Any = 1
  println(a == 1)
  println(P(1) == P(2))
  println(Color.Red == Color.Green)
  println(Some(P(1)) == Some(P(2)))
  println(Ids("a") == Ids("b"))
  println(Der.X == Color.Red)
