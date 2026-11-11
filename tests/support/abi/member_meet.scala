// TypeErasure.scala:767-779, 901-911, 1043-1052: a type reference follows the
// joint member information, even when its symbol represents only one side.
// abi: Api#two Api#three Api#twoResult Api#threeResult Api#twoArray Api#threeArray Api$#two Api$#three Api$#twoResult Api$#threeResult Api$#twoArray Api$#threeArray
package meetmin
trait A { type T <: CharSequence }
trait B { type T <: java.io.Serializable }
trait C { type T <: String }
object Api:
  def two(c: B & A)(x: c.T): Int = 1
  def three(c: C & A & B)(x: c.T): Int = 2
  def twoResult(c: A & B)(x: c.T): c.T = x
  def threeResult(c: B & A & C)(x: c.T): c.T = x
  def twoArray(c: B & A)(x: Array[c.T]): Array[c.T] = x
  def threeArray(c: C & A & B)(x: Array[c.T]): Array[c.T] = x
