import scala.compiletime.codeOf
import scala.compiletime.testing.typeCheckErrors

class Parent:
  override def toString: String = "parent"
class Child extends Parent:
  def shown: String = codeOf(super.toString)

inline def copied(inline value: String): String = codeOf(value)

@main def run(): Unit =
  val s = "abc"
  println(codeOf(s.isEmpty))
  println(codeOf(s.toUpperCase))
  println(new Child().shown)
  println(codeOf(s.length))
  println(copied(s.toUpperCase))
  println(s.isEmpty)
  println(s.toUpperCase)
  println(s.length)
  println(typeCheckErrors("""import scala.compiletime.requireConst; def f(s: String) = requireConst(s.isEmpty)""").map(_.message).mkString("\n"))
  println(typeCheckErrors("""import scala.compiletime.requireConst; def f(s: String) = requireConst(s.toUpperCase)""").map(_.message).mkString("\n"))
  println(typeCheckErrors("""import scala.compiletime.requireConst; class C { def f = requireConst(super.toString) }""").map(_.message).mkString("\n"))
  println(typeCheckErrors("""import scala.compiletime.requireConst; def f(s: String) = requireConst(s.length)""").map(_.message).mkString("\n"))
  println(typeCheckErrors("""import scala.compiletime.testing.typeCheckErrors; def f(s: String) = typeCheckErrors(s.isEmpty.toString)""").map(_.message).mkString("\n"))
  println(typeCheckErrors("""import scala.compiletime.testing.typeCheckErrors; def f(s: String) = typeCheckErrors(s.toUpperCase)""").map(_.message).mkString("\n"))
  println(typeCheckErrors("""import scala.compiletime.testing.typeCheckErrors; class C { def f = typeCheckErrors(super.toString) }""").map(_.message).mkString("\n"))
  println(typeCheckErrors("""import scala.compiletime.testing.typeCheckErrors; def f(s: String) = typeCheckErrors(s.length.toString)""").map(_.message).mkString("\n"))
  println(codeOf(s.charAt(1)))
  println(codeOf(s.substring(1)))
  println(codeOf(s.substring(1, 2)))
