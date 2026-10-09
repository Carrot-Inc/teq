// An inline call a quote keeps for the site expands there into the node of the call, which takes
// the records of the expansion: the type test a transparent method's body holds, the call an
// ordinary one stands for.
import scala.quoted.*

object DeferredInline:
  transparent inline def pred(x: Any): Boolean = x.isInstanceOf[String]
  inline def plain(x: Any): Boolean = x.isInstanceOf[Int]

  transparent inline def viaTransparent(x: Any): Boolean = ${ viaTransparentImpl('x) }
  def viaTransparentImpl(x: Expr[Any])(using Quotes): Expr[Boolean] = '{ pred($x) }

  transparent inline def viaPlain(x: Any): Boolean = ${ viaPlainImpl('x) }
  def viaPlainImpl(x: Expr[Any])(using Quotes): Expr[Boolean] = '{ plain($x) }
