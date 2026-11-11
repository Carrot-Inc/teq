// A retained case renews its local inline val together with the imported alias.
import scala.compiletime.{constValue, erasedValue}
inline def textOf[T]: String =
  class Local:
    def text: String = inline erasedValue[T] match
      case _: Option[a] =>
        inline val text = constValue[a]
        import text.{toString as render}
        render + text.toString
      case _ => "empty"
  new Local().text
@main def run(): Unit =
  println(textOf[Option["abc"]])
  println(textOf[Option["hello"]])
