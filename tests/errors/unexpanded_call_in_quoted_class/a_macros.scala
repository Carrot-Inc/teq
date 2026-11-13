// command: build
// expect: error: the inline call quoted.Doubling.twice in
// expect: was not expanded
// A macro's quote declares a class whose method calls an inline method. The class is copied from the quote
// with that call left as a call, so the output would call a method nothing defines. scalac prints `8`.
package quoted

import scala.quoted.*

object Doubling:
  inline def twice(inline x: Int): Int = x + x

object Making:
  inline def made: Int = ${ madeImpl }
  def madeImpl(using Quotes): Expr[Int] = '{ class Held { def of(z: Int) = Doubling.twice(z) }; new Held().of(4) }
