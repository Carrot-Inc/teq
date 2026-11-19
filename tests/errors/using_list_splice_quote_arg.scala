// A quote among a macro implementation's arguments is splice code too: a short `using` list in it keeps teq's refusal
// (scalac types the quote as any code and accepts it: a row of its own), and with a splice in it scalac refuses
// ("Quoted argument of macros may not have splices"). The implementation's own body, typed on demand from the
// splice, is not splice code: its short list is searched, as scalac searches it.
// expect: 16:40: error: missing argument for parameter b
// expect: 18:57: error: missing argument for parameter b
import scala.quoted.*
class A(val v: Int)
class B(val v: Int)
given B = B(2)
def take(x: Int)(using a: A, b: B): Int = x + a.v + b.v
object Impls:
  def impl(e: Expr[Int])(using q: Quotes): Expr[Int] = e
  def late(using q: Quotes) = { val n = take(1)(using A(1)); Expr(n) }
object QuoteArg:
  inline def m: Int = ${ Impls.impl('{ take(1)(using A(1)) }) }
object QuoteArgSplice:
  inline def m: Int = ${ Impls.impl('{ ${ Impls.impl('{ take(1)(using A(1)) }) } }) }
object OnDemand:
  inline def m: Int = ${ Impls.late }
