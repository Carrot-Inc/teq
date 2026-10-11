// A macro's reflective calls of array overloads, through scala-library's `Selectable` on a JVM
// build (`getMethod(name, paramTypes*)` over the class literals the interpreter names by their
// elements) and the lean std's on JavaScript: each reaches its own alternative.
import scala.quoted.*
import scala.reflect.Selectable.reflectiveSelectable

class B:
  def f(x: Array[Int]): String = "ints"
  def f(x: Array[String]): String = "strings"

object M:
  inline def result: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    val b = reflectiveSelectable(new B)
    val a = b.applyDynamic("f", classOf[Array[Int]])(Array(1))
    val s = b.applyDynamic("f", classOf[Array[String]])(Array("s"))
    Expr(s"$a $s")
