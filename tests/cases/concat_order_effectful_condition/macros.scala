import scala.quoted.*

object Conds:
  def cond(): true = { println("  c"); true }

object Macros:
  inline def condHole(inline x: String, inline y: String): String = ${ condHoleImpl('x, 'y) }
  def condHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ (if Conds.cond() then $x else "") + $y }
