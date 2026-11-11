import scala.compiletime.{constValue as constant, erasedValue, summonInline}

object Library:
  inline def number: Int = constant[7]
  inline def qualified: Int = scala.compiletime.constValue[8]
  inline def kind[A]: String = inline erasedValue[A] match
    case _: Int => "int"
    case _ => "other"
  inline def chosen[A]: A = summonInline[A]

object Main:
  given Int = 11
  def main(args: Array[String]): Unit =
    println(Library.number)
    println(Library.qualified)
    println(Library.kind[Int])
    println(Library.chosen[Int])
