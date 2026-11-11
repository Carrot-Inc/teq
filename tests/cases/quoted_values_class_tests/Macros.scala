// A quoted expression and a quoted type are values of `scala.quoted.Expr` and `scala.quoted.Type`: a typed
// pattern over one tests its class as over any value, inside a macro's run as anywhere.
import scala.quoted.*

object Probe:
  inline def show(inline n: Int): String = ${ showImpl('n) }

  def isExpr(x: Any): Boolean = x match
    case e: Expr[?] => true
    case _ => false

  def isType(x: Any): Boolean = x match
    case t: Type[?] => true
    case _ => false

  def showImpl(n: Expr[Int])(using Quotes): Expr[String] =
    val pair: (String, Expr[Int]) = ("a", n)
    val typed = pair match
      case (s, e: Expr[Int]) => "typed"
    Expr(isExpr(n).toString + " " + isExpr("s") + " " + isType(Type.of[Int]) + " " + isType(n) + " " + typed)
