// A short `using` list of the call a macro's splice makes is not completed by a search: that call's arguments are
// taken as written (scalac's Splicer.checkValidMacroBody refuses any other, "Malformed macro parameter", which is a part
// of its own), so teq's own refusal stays, the parameter missing.
// expect: 14:29: error: missing argument for parameter t
// expect: 16:29: error: missing argument for parameter u
// expect: 18:29: error: missing argument for parameter t
// expect: 20:29: error: missing argument for parameter t
// expect: 22:41: error: missing argument for parameter t
import scala.quoted.*
object Impls:
  def impl(using q: Quotes, t: Type[Int]): Expr[String] = Expr(Type.show[Int])
  def impl2(using q: Quotes)(using t: Type[Int], u: Type[String]): Expr[String] = Expr("two")
object Review:
  inline def m: String = ${ Impls.impl(using summon[Quotes]) }
object SecondList:
  inline def m: String = ${ Impls.impl2(using summon[Quotes])(using Type.of[Int]) }
object Named:
  inline def m: String = ${ Impls.impl(using q = summon[Quotes]) }
object Other:
  inline def n: String = ${ Impls.impl(using summon[Quotes]) }
object Transparent:
  transparent inline def t: String = ${ Impls.impl(using summon[Quotes]) }
