import scala.compiletime.erasedValue

inline def classify[A]: Int = inline erasedValue[A] match
  case _: Int => 11
  case _ => 22

inline def choose[T](useDefault: Boolean): Int = inline erasedValue[T] match
  case _: List[a] =>
    def nested(x: Int = inline if true then classify[a] else 99): Int = x
    if useDefault then nested() else nested(44)
  case _ => 33

@main def run(): Unit =
  println(choose[List[Int]](true))
  println(choose[List[String]](true))
  println(choose[List[Int]](false))
  println(choose[String](true))
