package inspection
import scala.quoted.*
inline def holderIs[A](inline name: String): Boolean = ${holderIsImpl[A]('name)}
def holderIsImpl[A: Type](name: Expr[String])(using Quotes): Expr[Boolean] =
  import quotes.reflect.*
  val holder = Expr.summon[A].get.asTerm.symbol.owner
  Expr(holder.name.startsWith(name.valueOrAbort) && holder.flags.is(Flags.Module) && holder.flags.is(Flags.Synthetic))
