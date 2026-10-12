// A macro without a declared result type has the type its splice's Expr holds, over its type parameters,
// against which a call infers its type arguments before the macro runs (dotty's Namer infers it from the
// right-hand side): lila's given ConfigLoader[Config] = AutoConfig.loader.
import scala.quoted.*

trait Loader[A]:
  def load(s: String): A

object AutoLoader:
  inline def loader[T] = ${ impl[T] }
  private def impl[T](using Quotes)(using tpe: Type[T]) =
    import quotes.reflect.*
    val name = TypeRepr.of[T].typeSymbol.name
    '{ new Loader[T] { def load(s: String): T = throw new Exception(${ Expr(name) } + ":" + s) } }
