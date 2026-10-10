// A quote that applies an inline parameter (`'{ (c: Ctx) ?=> lambda(using c) }`, monocle's `Focus`):
// the expansion reduces the argument's closure there before the macro reads the quote
// (`InlineTyper.typedApply`'s `BetaReduce`), so the macro sees a lambda whose body selects the field.
// The stored body names the parameter's proxy in its quotes: the check holds the method for the
// retype path, which types the quote with the argument in place.
import scala.quoted.*
trait Ctx
class Mk[S]:
  transparent inline def apply[T](inline lambda: Ctx ?=> S => T): Any = ${ M.impl[S, T]('{ (c: Ctx) ?=> lambda(using c) }) }
object G:
  def apply[S] = new Mk[S]
object M:
  def impl[S: Type, T: Type](lambda: Expr[Ctx ?=> S => T])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def shape(t: Term): String = t match
      case Inlined(_, _, b) => shape(b)
      case Block(List(DefDef(_, _, _, Some(body))), Closure(_, _)) => "Lambda(" + shape(body) + ")"
      case Typed(e, _) => shape(e)
      case Block(Nil, e) => shape(e)
      case Select(_, n) => "Select(" + n + ")"
      case Apply(f, _) => "Apply(" + shape(f) + ")"
      case _ => "other"
    Expr(shape(lambda.asTerm))
