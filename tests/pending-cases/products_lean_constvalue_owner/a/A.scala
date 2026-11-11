package cva

object A:
  inline def five: Int = scala.compiletime.constValue[5]
  inline def six: Int =
    import scala.compiletime.constValue
    constValue[6]
  inline def kind[T]: Int = inline scala.compiletime.erasedValue[T] match
    case _: Int => 1
    case _ => 2
