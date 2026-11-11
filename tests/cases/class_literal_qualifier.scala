// jars: scala-library
import scala.compiletime.codeOf
class Box[A]
object LiteralQualifier:
  def qualifier(): scala.Predef.type =
    println("receiver evaluated")
    scala.Predef
inline def qualifiedCode[T]: String = codeOf(LiteralQualifier.qualifier().classOf[Box[T]])
@main def run(): Unit =
  println(LiteralQualifier.qualifier().classOf[String].isPrimitive)
  println(codeOf(LiteralQualifier.qualifier().classOf[String]))
  println(qualifiedCode[String])
