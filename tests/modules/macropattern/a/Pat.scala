package mpa

import scala.quoted.*

// Quote patterns of the upstream's macros, matched against the downstream's arguments at its
// expansions: holes, an ascribed hole, a type variable bound implicitly and one declared, a
// case class applied by its name, a type pattern.
object Pat:
  def plain(n: Int): Int = n

  inline def kind(inline e: Any): String = ${ kindImpl('e) }
  def kindImpl(e: Expr[Any])(using Quotes): Expr[String] = e match
    case '{ ($x: Int) + ($y: Int) } => Expr("add")
    case '{ ($l: List[t]).head } => Expr("head of " + Type.show[t])
    case '{ type t <: AnyVal; Some($v: t) } => Expr("some " + Type.show[t])
    case '{ Pat.plain($a) } => Expr("plain " + a.valueOrAbort)
    case _ => Expr("other")

  inline def typeKind[T]: String = ${ typeKindImpl[T] }
  def typeKindImpl[T: Type](using Quotes): Expr[String] = Type.of[T] match
    case '[List[e]] => Expr("list of " + Type.show[e])
    case '[Int] => Expr("int")
    case _ => Expr("other")
