// A quote that names a method whose result is inferred types that body on demand, as its own
// unit: outside the quote, so the plain inline call in it expands as the walk expands it, where a
// quote keeps its inline calls for the site it is spliced at (the body kept `N.one` callable, a
// method no target has). An anonymous class made in the quote keeps the quote's level.
import scala.quoted.*
object M:
  inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = '{ N.demanded + N.twice(2) }
  inline def shown: String = ${ show }
  def show(using Quotes): Expr[String] = '{ new Shown { def text = N.label + ${ Expr("!") } }.text }
trait Shown { def text: String }
object N:
  def demanded = one
  def twice(n: Int) = one + n
  def label = name
  inline def one: Int = 1
  inline def name: String = "n"
