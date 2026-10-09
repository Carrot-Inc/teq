// An infix operator on a primitive receiver whose right operand is no number reaches an
// extension method, as the dotted call does.
final class Money(val cents: Long):
  override def toString: String = s"Money($cents)"

extension (n: Int | Long)
  def *(that: Money): Money =
    val factor = n match
      case i: Int => i.toLong
      case l: Long => l
    Money(factor * that.cents)

final class Vec(val x: Int, val y: Int):
  override def toString: String = s"Vec($x, $y)"

extension (d: Double) def *(v: Vec): Vec = Vec((d * v.x).toInt, (d * v.y).toInt)
extension (n: Int)
  def +(v: Vec): Vec = Vec(n + v.x, n + v.y)
  def <(v: Vec): Boolean = n < v.x && n < v.y
extension (c: Char) def *(v: Vec): String = c.toString * v.x

@main def main(): Unit =
  println(3 * Money(250))
  println(3.*(Money(250)))
  println(7L * Money(2))
  println(2.5 * Vec(2, 4))
  println(1 + Vec(1, 2))
  println(1 < Vec(2, 3))
  println(5 < Vec(2, 3))
  println('a' * Vec(3, 0))
  var count = 4
  println(count * Money(3))
  val counts = Array(1, 2)
  println(counts(1) * Money(10))
  println(3 * 4)
  println(3 + "x")
