package mlib
import scala.deriving.Mirror
import scala.quoted.*

trait Info[E]:
  def text: String
object Info:
  transparent inline def derived[E]: Info[E] = ${ Macros.info[E] }
  inline def probe[E]: String = ${ Macros.probe[E] }

object Macros:
  def probe[E: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val sum = Expr.summon[Mirror.SumOf[E]].isDefined
    val of = Expr.summon[Mirror.Of[E]].isDefined
    val prod = Expr.summon[Mirror.ProductOf[E]].isDefined
    Expr(s"${TypeRepr.of[E].show}: sum=$sum of=$of product=$prod")
  def info[E: Type](using Quotes): Expr[Info[E]] =
    val msg = probe[E]
    '{ new Info[E] { def text: String = $msg } }
