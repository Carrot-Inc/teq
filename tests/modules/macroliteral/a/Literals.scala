package mla

import scala.quoted.*

// A literal interpolator as a library of them shapes one: the macro's implementation inherited
// from a trait, which calls the object's `validate`, which calls the model's parsing.
trait Literal[A]:
  def validate(s: String)(using Quotes): Either[String, Expr[A]]

  def apply(ctx: Expr[StringContext], args: Expr[Seq[Any]])(using Quotes): Expr[A] =
    ctx.value match
      case Some(sc) if sc.parts.size == 1 =>
        validate(sc.parts.head) match
          case Right(e) => e
          case Left(msg) => quotes.reflect.report.errorAndAbort(msg)
      case _ => quotes.reflect.report.errorAndAbort("a literal takes no arguments")

object Literals:
  extension (inline ctx: StringContext)
    inline def email(inline args: Any*): Email = ${ EmailLiteral('ctx, 'args) }

  object EmailLiteral extends Literal[Email]:
    def validate(s: String)(using Quotes): Either[String, Expr[Email]] =
      Email.fromStringEither(s).map(_ => '{ Email.fromString(${ Expr(s) }).get })
