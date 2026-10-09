// What fills a quote's hole takes the layers written around the hole: a cast, an ascription,
// the name of the argument it is passed as.
import scala.quoted.*

object HoleLayers:
  def g(a: Int, b: Any): Int = a

  transparent inline def cast(x: Any): String = ${ castImpl('x) }
  def castImpl(x: Expr[Any])(using Quotes): Expr[String] = '{ $x.asInstanceOf[String] }

  transparent inline def ascribed(x: Int): Any = ${ ascribedImpl('x) }
  def ascribedImpl(x: Expr[Int])(using Quotes): Expr[Any] = '{ ($x: Any) }

  transparent inline def named(x: Int): Int = ${ namedImpl('x) }
  def namedImpl(x: Expr[Int])(using Quotes): Expr[Int] = '{ g(b = $x, a = 1) }
