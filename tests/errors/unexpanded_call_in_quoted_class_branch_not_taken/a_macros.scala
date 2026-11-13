// command: build
// expect: error: the inline call quoted.Doubling.twice in
// expect: was not expanded
// The class of `unexpanded_call_in_quoted_class`, its inline call in a branch this run does not take: the
// build is refused all the same, since the method written would hold a call of a method nothing defines
// whatever a run takes. scalac prints `4`.
package quoted

import scala.quoted.*

object Doubling:
  inline def twice(inline x: Int): Int = x + x

object Making:
  inline def made: Int = ${ madeImpl }
  def madeImpl(using Quotes): Expr[Int] = '{ class Held { def of(z: Int) = if z > 100 then Doubling.twice(z) else z }; new Held().of(4) }
