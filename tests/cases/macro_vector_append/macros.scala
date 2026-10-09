import scala.quoted.*

// A macro that appends to a Vector while it expands: the interpreter runs `Vector.:+` on a
// JavaScript build too, where the std's own JavaScript templates have no engine to run in.
object Macros:
  inline def appended(inline n: Int): String = ${ appendedImpl('n) }

  def appendedImpl(n: Expr[Int])(using Quotes): Expr[String] =
    val count = n.valueOrAbort
    var v = Vector.empty[Int]
    var i = 0
    while i < count do
      v = v :+ i * i
      i += 1
    val held = v
    v = v :+ -1
    val text = held.length.toString + " " + v.length + " " + held.take(3).mkString(",") + " " + v.last + " " + held.updated(0, 9).head + " " + (held.drop(count - 2) :+ 7).mkString(",")
    Expr(text)
