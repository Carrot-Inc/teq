// Two files each calling the other's transparent macro: each unit suspends at typing
// (`Inliner.expandMacro` in the typer), which leaves the run no unit, as dotty's
// `Run.checkSuspendedUnits` reports after `TyperPhase` (scalac 3.8.4 prints the same three
// paragraphs and its hint).
import scala.quoted.*
object A:
  transparent inline def m: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(1)
  val usesB: Int = B.m
