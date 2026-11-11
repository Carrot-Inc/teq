import scala.compiletime.erasedValue
inline def choose[T]: Int = inline erasedValue[T] match
  case _: List[a] =>
    def nested(x: Int = inline erasedValue[a] match
      case _: Int => 11
      case _ => 22
    ): Int = x
    nested()
  case _ => 33
@main def run(): Unit =
  println(choose[List[Int]])
  println(choose[List[String]])
