// A macro whose implementation is defined in the file of the call: scalac runs a macro's code
// compiled and rejects the call.
// expect: Cannot call macro method showImpl defined in the same source file
import scala.quoted.*

object Main:
  inline def show(x: Int): String = ${ showImpl('x) }
  def showImpl(x: Expr[Int])(using Quotes): Expr[String] = Expr("ab")
  def main(args: Array[String]): Unit = println(show(1))
