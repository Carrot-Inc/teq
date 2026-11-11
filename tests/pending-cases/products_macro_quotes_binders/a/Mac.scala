package qra

import scala.quoted.*

object Mac:
  inline def nameOf[T]: String = ${ nameImpl[T] }
  def nameImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val tpe = TypeRepr.of[T]
    Expr(tpe.show)
  def other[T: Type](using Quotes): Expr[Int] =
    import quotes.reflect.*
    val tpe = TypeRepr.of[T]
    Expr(tpe.show.length)
