// The inline `this` decision on a tree a macro's quote splices back (`'{ val r = $x; r }`): the argument's inline
// call keeps its prefix, a cast or an ascription of `this`, through the quote's copy, decided as from source
// (Use.scala).
import scala.quoted.*
object Mac:
  inline def keep(inline x: Int): Int = ${ keepImpl('x) }
  def keepImpl(x: Expr[Int])(using Quotes): Expr[Int] = '{ val r = $x; r }
