package sample
import scala.quoted.*
opaque type Token = String
object Token:
  def make(text: String): Token = text
object Builder:
  def literal(ctx: Expr[StringContext])(using Quotes): Expr[Token] = '{ Token.make($ctx.parts.mkString) }
extension (inline ctx: StringContext)
  inline def checked(inline args: Any*): Token = ${ Builder.literal('ctx) }
inline def consume(inline args: (String | Token)*): Int = args.size
