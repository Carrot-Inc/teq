// A class test against `scala.quoted.Expr` and `scala.quoted.Type` inside a macro's run: a quoted expression or type
// is one, a reflection `Term` or `TypeRepr` is not.
import scala.quoted.*

object Probe:
  inline def kinds: String = ${ kindsImpl }

  def kind(x: Any): String = x match
    case _: Expr[?] => "expr"
    case _: Type[?] => "type"
    case _ => "other"

  def kindsImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val term: Any = Literal(IntConstant(1))
    val expr: Any = '{ 1 }
    val repr: Any = TypeRepr.of[Int]
    val tpe: Any = Type.of[Int]
    Expr(List(kind(term), kind(expr), kind(repr), kind(tpe), kind(Literal(IntConstant(1)).asExpr)).mkString(" "))
