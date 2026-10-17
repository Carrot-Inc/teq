// A given conversion's instance variable held contravariantly by the view's type (`Sink[-A]` of
// `conv[A]: Conversion[Raw, Sink[A]]`) is instantiated from above, as `interpolateTypeVars` takes a
// contravariant occurrence (Inferencing.scala 718-745): `Sink[Any]`, whose `put` takes a
// `String`, and which an expected `Sink[String]` takes, as scalac's `conv[Any]`.
package viewcontravariantvariable

import scala.language.implicitConversions

final case class Raw(n: Int)
class Sink[-A]:
  def put(a: A): Int = 7
  def self: Sink[A] = this
given conv[A]: Conversion[Raw, Sink[A]] = r => new Sink[A]

object Main:
  def main(args: Array[String]): Unit =
    println(Raw(1).put("x"))
    val s: Sink[String] = Raw(2).self
    println(s.put("y") + 1)
