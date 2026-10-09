// A Java-defined method takes () whether or not it declares an empty parameter list, and
// `toString()` picks the nullary alternative next to `toString(radix)`.
package javaparensoverloads

import java.math.BigInteger

class Radix(v: Int):
  override def toString: String = "Radix" + v
  def toString(radix: Int): String = Integer.toString(v, radix)

@main def main(): Unit =
  val xs = java.util.List.of("a", "b")
  println(xs.size())
  println(xs.size)
  println(xs.isEmpty())
  val n = BigInteger.valueOf(255)
  println(n.toString())
  println(n.toString)
  println(n.toString(16))
  println(n.signum())
  println(n.negate().abs())
  val r = Radix(10)
  println(r)
  println(r.toString(2))
  println(List(r))
