// A macro applies a generic method it selected without its `TypeApply`, and a generic class's
// constructor selected on `New(TypeIdent(C))`: their types are `PolyType`s, which no `Apply` takes
// (`TypeAssigner.assignType` on `fun.tpe.widen`), refused with scalac's message.
import scala.quoted.*
object M:
  def f[A](x: A): A = x
class Gen[A](val a: A)
object Mac:
  inline def test: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val f = Select.unique('{ M }.asTerm, "f")
    Apply(f, List('{ 1 }.asTerm)).asExprOf[Int]
  inline def ctor: Any = ${ ctorImpl }
  def ctorImpl(using Quotes): Expr[Any] =
    import quotes.reflect.*
    val sym = TypeRepr.of[Gen[Int]].typeSymbol
    Apply(Select(New(TypeIdent(sym)), sym.primaryConstructor), List('{ 1 }.asTerm)).asExprOf[Any]
