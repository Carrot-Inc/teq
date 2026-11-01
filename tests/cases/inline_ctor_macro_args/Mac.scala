// A macro's argument naming the class's own parameter accessor in its constructor code: placed in a lambda (`lam`) it
// reads the parameter (dotty's `Constructors.intoConstr` over the `Inlined` tree), in an anonymous class's body
// (`wrap`) the accessor (`noDirectRefsFrom` stops at the class), a compound argument there too (`wrap(s + 1)`); from a
// method (`viaMethod`) the accessor.
import scala.quoted.*
abstract class Fn:
  def apply(): Int
object Mac:
  inline def wrap(inline x: Int): Fn = ${ wrapImpl('x) }
  def wrapImpl(x: Expr[Int])(using Quotes): Expr[Fn] = '{ new Fn { def apply() = $x } }
  inline def lam(inline x: Int): () => Int = ${ lamImpl('x) }
  def lamImpl(x: Expr[Int])(using Quotes): Expr[() => Int] = '{ () => $x }
