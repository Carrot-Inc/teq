// A short `using` list anywhere in a macro's splice code is not completed by a search, whatever wraps the call: braces,
// a block over lines, parentheses, an ascription, an `if`, a `val` returned, an argument of another call (scalac refuses
// each, "Malformed macro parameter" or "Malformed macro"); teq keeps its own refusal. A splice inside a quote of an
// ordinary method is not a macro's splice: its short list is searched, as scalac searches it (QuoteControl).
// expect: 18:31: error: missing argument for parameter t
// expect: 21:5: error: missing argument for parameter t
// expect: 24:30: error: missing argument for parameter t
// expect: 26:29: error: missing argument for parameter t
// expect: 28:42: error: missing argument for parameter t
// expect: 28:80: error: missing argument for parameter t
// expect: 30:39: error: missing argument for parameter t
// expect: 32:40: error: missing argument for parameter t
import scala.quoted.*
object Impls:
  def impl(using q: Quotes, t: Type[Int]): Expr[String] = Expr(Type.show[Int])
  def wrap(e: Expr[String])(using Quotes): Expr[String] = e
object InnerBraces:
  inline def m: String = ${ { Impls.impl(using summon[Quotes]) } }
object MultiLine:
  inline def m: String = ${
    Impls.impl(using summon[Quotes])
  }
object Parens:
  inline def m: String = ${ (Impls.impl(using summon[Quotes])) }
object Ascribed:
  inline def m: String = ${ Impls.impl(using summon[Quotes]): Expr[String] }
object IfSplice:
  inline def m: String = ${ if true then Impls.impl(using summon[Quotes]) else Impls.impl(using summon[Quotes]) }
object ValReturned:
  inline def m: String = ${ { val e = Impls.impl(using summon[Quotes]); e } }
object ArgOf:
  inline def m: String = ${ Impls.wrap(Impls.impl(using summon[Quotes])) }
object QuoteControl:
  def f(using Quotes): Expr[String] = '{ ${ Impls.impl(using summon[Quotes]) } + "!" }
