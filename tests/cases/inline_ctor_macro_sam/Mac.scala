// A macro's argument naming the class's own parameter accessor in its constructor code, placed in a lambda of an
// abstract class's SAM type: dotty's `ExpandSAMs` keeps the lambda's body a local function of the constructor, which
// `Constructors.intoConstr` reads the parameter in (`3`); teq's class for the lambda captures the parameter.
import scala.quoted.*
abstract class Fn:
  def apply(): Int
object Mac:
  inline def sam(inline x: Int): Fn = ${ samImpl('x) }
  def samImpl(x: Expr[Int])(using Quotes): Expr[Fn] = '{ ((() => $x): Fn) }
