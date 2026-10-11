// A macro calls members structurally (scala-library's `Selectable`): an alternative of an
// overloaded name by its parameters' classes, a val through `selectDynamic`, a method explicitly
// through `applyDynamic`. A JVM build's macro runs scala-library's body, which reflects through
// `getField`, `getMethod` with the parameters' classes and `invoke`; the others the std's.
import scala.quoted.*
import scala.reflect.Selectable.reflectiveSelectable

class Box(val size: Int):
  def f(x: Int): String = s"int $x"
  def f(x: String): String = s"string $x"

object Macros:
  inline def calls: String = ${ callsImpl }

  def callsImpl(using Quotes): Expr[String] =
    val i: { def f(x: Int): String } = new Box(3)
    val s: { def f(x: String): String } = new Box(3)
    val v: { val size: Int } = new Box(3)
    val b = reflectiveSelectable(new Box(4))
    Expr(List(i.f(1), s.f("a"), v.size.toString, b.applyDynamic("f", classOf[String])("b").toString).mkString(", "))
