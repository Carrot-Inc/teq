// An application's symbol is its function's (dotty's `GenericApply` forwards its denotation,
// `Trees`): the first `Apply` of a method of three clauses, a using one in the middle, is of the
// method's symbol, its tree the method's definition.
import scala.quoted.*
object Methods:
  def f(a: Int)(using b: String)(c: Int): String = s"$a $b $c"
object Mac:
  inline def test: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val f = Select.unique('{ Methods }.asTerm, "f")
    val first = Apply(f, List('{ 1 }.asTerm))
    val second = Apply(first, List('{ "x" }.asTerm))
    val last = Apply(second, List('{ 2 }.asTerm))
    val defName = first.symbol.tree match
      case d: DefDef => d.name
      case _ => "no definition"
    val info = Expr(first.show + ";" + first.symbol.name + ";" + second.symbol.name + ";" + defName + ";" + first.isExpr + ";")
    '{ $info + ${ last.asExprOf[String] } }
