// Two files each calling the other's macro: every unit of the run suspends again, as dotty's
// `Run.checkSuspendedUnits` reports (scalac 3.8.4 prints the same three paragraphs and the hint of
// its -Xprint-suspension option).
import scala.quoted.*
object A:
  inline def m: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(1)
  val usesB: Int = B.m
