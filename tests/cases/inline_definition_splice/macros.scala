import scala.quoted.*

// A macro's splice is stored at the definition as scalac keeps it, the context function of the
// `Quotes` its code takes (`${ (using q: Quotes) => impl(q) }`), whose binder a copy renames; the
// census checks both. Where `Unit` is declared any `Expr` is the splice's code,
// its value discarded, as scalac 3.8.4 accepts it; that `Expr[Any]` is also what infers the code's
// type arguments (`same('{1})` takes `A = Int`, not `Unit`), at the definition and at a call.
object M:
  inline def good: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(1)
  inline def done: Unit = ${ implAny }
  def implAny(using Quotes): Expr[Any] = '{ () }
  inline def inferred: Unit = ${ same('{1}) }
  inline def called: Unit = ${ same('{ "ab".length }) }
  def same[A](a: Expr[A])(using Quotes): Expr[A] = a
