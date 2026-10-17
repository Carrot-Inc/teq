// A given conversion's instance variables interpolated as dotty's `interpolateTypeVars` does,
// their variances read through the bounds (`Inferencing.variances`' `propagate`, 555-597): with
// `Box[+A, B <: A]` the nonvariant `B` makes `A` nonvariant, so both stay open for the member,
// whose argument fixes them (`[String, String]`), as does a variable named only in another's
// bound (`B <: List[A]`); the instantiation of the rest waits on what it depends on
// (`instantiateTypeVars`).
package viewboundvariance

import scala.language.implicitConversions

case class Raw()

class CoInv[+A, B <: A]:
  def put(x: B): Int = 7
given coInv[A, B <: A]: Conversion[Raw, CoInv[A, B]] with
  def apply(r: Raw): CoInv[A, B] = new CoInv[A, B]

case class Raw2()
class InvCo[A, +B <: A]:
  def put(x: A): Int = 8
given invCo[A, B <: A]: Conversion[Raw2, InvCo[A, B]] with
  def apply(r: Raw2): InvCo[A, B] = new InvCo[A, B]

case class Raw3()
class InvContra[A, -B >: A]:
  def put(x: A): Int = 9
given invContra[A, B >: A]: Conversion[Raw3, InvContra[A, B]] with
  def apply(r: Raw3): InvContra[A, B] = new InvContra[A, B]

case class Raw4()
class Omitted[B]:
  def put(x: B): Int = 10
given omitted[A, B <: List[A]]: Conversion[Raw4, Omitted[B]] with
  def apply(r: Raw4): Omitted[B] = new Omitted[B]

object Main:
  def main(args: Array[String]): Unit =
    println(Raw().put("x"))
    println(Raw2().put("y"))
    println(Raw3().put("z"))
    println(Raw4().put(List(1)))
