// A generic method a macro selects, and a generic class's constructor selected on its `New`, are
// applied once a `TypeApply` has instantiated their `PolyType`s (`TypeAssigner.assignType`);
// without it they are refused (tests/errors/macro_apply_poly_unapplied).
import scala.quoted.*
object M:
  def f[A](x: A): A = x
class Gen[A](val a: A)
object Mac:
  inline def test: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val f = Select.unique('{ M }.asTerm, "f")
    Apply(TypeApply(f, List(TypeTree.of[Int])), List('{ 1 }.asTerm)).asExprOf[Int]
  inline def ctor: Int = ${ ctorImpl }
  def ctorImpl(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val sym = TypeRepr.of[Gen[Int]].typeSymbol
    val made = Select(New(TypeIdent(sym)), sym.primaryConstructor).appliedToTypes(List(TypeRepr.of[Int])).appliedTo('{ 2 }.asTerm)
    '{ ${ made.asExprOf[Gen[Int]] }.a }
