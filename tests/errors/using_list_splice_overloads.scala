// An overloaded macro implementation called in a splice with a `using` list: the list does not narrow the alternatives
// there (scalac refuses the call, "Malformed macro parameter", the arguments taken as written), so teq keeps its own
// refusal, by the leading list and by a following one.
// expect: 13:29: error: Ambiguous overload.
// expect: 15:44: error: Ambiguous overload.
import scala.quoted.*
object Impls:
  def f(using q: Quotes): Expr[String] = Expr("one")
  def f(using q: Quotes, t: Type[Int]): Expr[String] = Expr("two")
  def g(e: Expr[Int])(using q: Quotes): Expr[String] = Expr("g1")
  def g(e: Expr[Int])(using q: Quotes, t: Type[Int]): Expr[String] = Expr("g2")
object Leading:
  inline def m: String = ${ Impls.f(using summon[Quotes]) }
object Following:
  inline def m(inline x: Int): String = ${ Impls.g('x)(using summon[Quotes]) }
