// A transparent macro, expanded by the typer, whose implementation runs `Kept`'s initialiser and a
// method of it: the interpreter runs their executable form, the calls the inlining phase would expand
// expanded first, as dotty compiles the macro's units through `Inlining` before running it.
import scala.quoted.*
object M:
  transparent inline def total: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(Kept.total + Kept.items.length)
