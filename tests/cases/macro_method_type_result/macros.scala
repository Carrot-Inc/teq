// `MethodType(names)(params, result)`: the result's function receives the method type with its
// parameters' types computed, as dotty's `MethodType` sets `paramInfos` before `resultTypeExp(this)`,
// so a result read off the binder's parameters is theirs.
import scala.quoted.*

object Macros:
  inline def check: Boolean = ${ checkImpl }
  def checkImpl(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    val mt = MethodType(List("x"))(_ => List(TypeRepr.of[String]), m => m.paramTypes.head)
    Expr(mt.resType =:= TypeRepr.of[String])

  inline def shown: String = ${ shownImpl }
  def shownImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val mt = MethodType(List("a", "b"))(_ => List(TypeRepr.of[Int], TypeRepr.of[Boolean]), m => m.paramTypes.last)
    Expr(mt.resType.show)
