package quotepatterns33

import scala.quoted.*

// Packaged with Scala 3.3.7: quote patterns encoded as QuoteMatching unapply in TASTy 28.3.

object BindOne:
  inline def show[A](inline a: A): String = ${ impl('a) }
  def impl[A: Type](a: Expr[A])(using Quotes): Expr[String] =
    import quotes.reflect.*
    a.asTerm.tpe.widen.asType match
      case '[l] => Expr(Type.show[l])

object BindTwo:
  inline def show[A, B](inline a: A, inline b: B): String = ${ impl('a, 'b) }
  def impl[A: Type, B: Type](a: Expr[A], b: Expr[B])(using Quotes): Expr[String] =
    import quotes.reflect.*
    (a.asTerm.tpe.widen.asType, b.asTerm.tpe.widen.asType) match
      case ('[l], '[r]) => Expr(Type.show[l] + " " + Type.show[r])

object IsBool:
  inline def test[A](inline a: A): Boolean = ${ impl('a) }
  def impl[A: Type](a: Expr[A])(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    a.asTerm.tpe.widen.asType match
      case '[Boolean] => Expr(true)
      case _          => Expr(false)

object TermVars:
  inline def test[A](inline a: A): String = ${ impl('a) }
  def impl[A: Type](a: Expr[A])(using Quotes): Expr[String] =
    a match
      case '{ type t; Some[`t`]($x: `t`) } => Expr("some")
      case _                               => Expr("other")

object TermPlain:
  inline def test(inline a: Int): String = ${ impl('a) }
  def impl(a: Expr[Int])(using Quotes): Expr[String] =
    a match
      case '{ ($x: Int) + 1 } => Expr("plus one")
      case _                  => Expr("other")

object IsQualifiedBool:
  inline def test[A](inline a: A): Boolean = ${ impl('a) }
  def impl[A: Type](a: Expr[A])(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    a.asTerm.tpe.widen.asType match
      case '[scala.Boolean] => Expr(true)
      case _                => Expr(false)

object TermStable:
  inline def test(inline a: Any): Boolean = ${ impl('a) }
  def impl(a: Expr[Any])(using Quotes): Expr[Boolean] =
    a match
      case '{ None } => Expr(true)
      case _         => Expr(false)

object SingletonType:
  inline def test[A]: Boolean = ${ impl[A] }
  def impl[A: Type](using Quotes): Expr[Boolean] =
    Type.of[A] match
      case '[None.type] => Expr(true)
      case _            => Expr(false)

// A pattern binding a type in a contravariant position, `Pipe[in, ..]` of `Pipe[-I, +E, +O]`, which scalac marks
// `@fromAbove` (tests/cases/quote_patterns_28_3_from_above.scala).
final case class Pipe[-I, +E, +O](name: String)

object Shapes:
  inline def shapeOf[E](inline x: Pipe[?, E, ?]): String = ${ impl[E]('x) }
  def impl[E: Type](x: Expr[Pipe[?, E, ?]])(using Quotes): Expr[String] =
    x match
      case '{ $l: Pipe[in, e & E, out] } => Expr(Type.show[in] + " | " + Type.show[e] + " | " + Type.show[out])
      case _ => Expr("no match")
