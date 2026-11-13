// jars: rounddemand-lib
// A macro's quote declares a class whose method calls a library's inline method that implements a member
// that is not inline. The call is left as a call, and links to the method scalac keeps for such an inline
// method (`twice$retainedBody` in the library's pickle, `twice` in its class file): the program prints 8.
package quoted

import scala.quoted.*

object Making:
  inline def made: Int = ${ madeImpl }
  def madeImpl(using Quotes): Expr[Int] = '{ class Held { def value: Int = rounddemand.Doubled.twice(4) }; new Held().value }
