// A macro passes named arguments: `Select.overloaded` and `Apply` take `NamedArg` trees, which
// go to the parameter they name, and a `NamedArg` built by the macro reads back.
import scala.quoted.*

object Macros:
  inline def setField[T](inline x: T, inline field: String, inline value: Any): T = ${ setFieldImpl[T]('x, 'field, 'value) }
  def setFieldImpl[T: Type](x: Expr[T], field: Expr[String], value: Expr[Any])(using Quotes): Expr[T] =
    import quotes.reflect.*
    val name = field.valueOrAbort
    Select.overloaded(x.asTerm, "copy", Nil, List(NamedArg(name, value.asTerm))).asExprOf[T]

  inline def callReversed(inline x: Any, inline method: String, inline a: Any, inline b: Any): Any = ${ callReversedImpl('x, 'method, 'a, 'b) }
  def callReversedImpl(x: Expr[Any], method: Expr[String], a: Expr[Any], b: Expr[Any])(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val recv = x.asTerm
    val name = method.valueOrAbort
    val params = recv.tpe.widen.typeSymbol.methodMember(name).head.paramSymss.head.map(_.name)
    val args = List(NamedArg(params(1), b.asTerm), NamedArg(params(0), a.asTerm))
    Apply(Select.unique(recv, name), args).asExpr

  inline def describeNamed(inline name: String, inline v: Int): String = ${ describeNamedImpl('name, 'v) }
  def describeNamedImpl(name: Expr[String], v: Expr[Int])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val arg = NamedArg(name.valueOrAbort, v.asTerm)
    val read = arg match
      case NamedArg(n, t) => n + " := " + t.show
    Expr(read + " | " + arg.name + " | " + arg.value.show + " | " + arg.show + " | " + arg.tpe.widen.show)
