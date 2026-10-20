// A transparent macro whose implementation is defined in the file of the call, met at typing time:
// dotty's `Inliner.expandMacro` reports the dependency in the unit's own source before it suspends
// the unit, and the report ends the build (no later run, no cycle).
// expect: 10:19: error: Cannot call macro method showImpl defined in the same source file
import scala.quoted.*

object Main:
  transparent inline def show(x: Int): String = ${ showImpl('x) }
  def showImpl(x: Expr[Int])(using Quotes): Expr[String] = Expr("ab")
  val s: String = show(1)
  def main(args: Array[String]): Unit = println(s)
