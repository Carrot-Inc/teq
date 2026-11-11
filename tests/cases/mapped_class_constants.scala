import scala.compiletime.codeOf
class Box[A]
inline def arrayCode[T]: String = codeOf(classOf[Array[T]])
inline def boxCode[T]: String = codeOf(classOf[Box[T]])
@main def run(): Unit =
  println(codeOf(classOf[Box[String]]))
  println(codeOf(classOf[Array[String]]))
  println(codeOf(classOf[Array[Array[Int]]]))
  println(arrayCode[String])
  println(arrayCode[Int])
  println(boxCode[String])
