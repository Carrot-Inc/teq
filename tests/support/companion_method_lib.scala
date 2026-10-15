// The jar side of tests/cases/companion_method_lib, compiled by scalac: an object holding an object and a
// method of one name, alternatives of the name as dotty's module val and method are: scalacheck's `Seed`
// (the companion of its private `case class apply` beside `def apply(s: Long)`, its bodies applying
// `Seed.apply.apply(..)`), and an explicit object beside a method.
package cmlib

sealed abstract class Seed extends Serializable:
  protected val a: Long
  protected val b: Long
  def next: Seed = Seed.apply(a ^ b, a + b)
  def value: Long = a * 31 + b

object Seed:
  private case class apply(a: Long, b: Long) extends Seed
  def apply(s: Long): Seed = apply(s, s + 1)
  def fromLongs(a: Long, b: Long): Seed = apply(a, b)

object Lookup:
  object find:
    def apply(a: Int, b: Int): Int = a * b
  def find(a: Int): Int = find(a, a)
