// jars: rounddemand-lib
// command: build
// expect: error: the inline call rounddemand.Tripled.thrice in Held.value was not expanded
// A macro's quote declares a class whose method calls a library's inline method that implements nothing:
// the call is left as a call, and neither the output nor the library's class file has a method for it.
// scalac prints `12`.
package quoted

import scala.quoted.*

object Making:
  inline def made: Int = ${ madeImpl }
  def madeImpl(using Quotes): Expr[Int] = '{ class Held { def value: Int = rounddemand.Tripled.thrice(4) }; new Held().value }
