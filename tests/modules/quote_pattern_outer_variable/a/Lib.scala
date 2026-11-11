package qpov.lib

import scala.quoted.*

object Same:
  inline def element[A, B]: Boolean = ${ elementImpl[A, B] }

  def elementImpl[A: Type, B: Type](using Quotes): Expr[Boolean] =
    Type.of[A] match
      case '[List[t]] =>
        Type.of[B] match
          case '[Option[`t`]] => Expr(true)
          case _ => Expr(false)
      case _ => Expr(false)

  inline def composed[A, B, C](inline f: A => B, inline g: B => C): A => C = ${ composedImpl[A, B, C]('f, 'g) }

  def composedImpl[A: Type, B: Type, C: Type](f: Expr[A => B], g: Expr[B => C])(using Quotes): Expr[A => C] =
    compose(f, g).asInstanceOf[Expr[A => C]]

  def compose(lhs: Expr[Any], rhs: Expr[Any])(using Quotes): Expr[Any] =
    lhs match
      case '{ $l: Function1[i, o] } =>
        rhs match
          case '{ $r: Function1[`o`, o2] } => '{ $l.andThen($r) }
