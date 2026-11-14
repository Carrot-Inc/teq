// A `for`'s alias that is a typed variable is the value definition `val x: T = e`: its right side is typed
// against the written type, its variable has that type, and nothing is tested.
import scala.language.implicitConversions
class Item
class Wrapped(val v: Int)
object Test:
  given Conversion[Int, Wrapped] = new Wrapped(_)
  def make(y: Any): Item = null
  // A null right side: no test of the written type stands in the way.
  def nulls: Int =
    val last = for (y <- List(1); l: Item = make(y)) yield if l == null then 1 else 2
    val before = for (y <- List(1); l: Item = make(y); z <- List(2)) yield if l == null then z else 0
    var seen = 0
    for (y <- List(1); l: Item = make(y)) do seen += (if l == null then 1 else 2)
    last.sum + before.sum + seen
  // A numeric widening of the right side, a second alias reading the first, an alias before the first generator.
  def widened: Double =
    val one = for (y <- List(1); l: Long = y) yield l + 1L
    val two = for (y <- List(1); l: Long = y; k: Double = l) yield k + 0.5
    val first =
      for
        l: Long = 2
        y <- List(1)
      yield l + y
    one.sum + two.sum + first.sum
  // The written type is the right side's expected type and the variable's type.
  def expected: Int =
    val lambda = for (y <- List(1); f: (Int => Int) = _ + y) yield f(2)
    val empty = for (y <- List(1, 2); l: Option[Int] = None; k: List[Int] = Nil) yield y + l.getOrElse(0) + k.sum
    val converted = for (y <- List(1); l: Wrapped = y) yield l.v + 1
    lambda.sum + empty.sum + converted.sum
  // A lowercase name of the written type is the type of that name.
  def named: Int =
    type elem = String
    val r = for (y <- List(1); l: List[elem] = List("a")) yield l.head.length
    r.sum
  // Aliases that stay patterns: a typed wildcard and a parenthesised typed variable test their type.
  def patterns: Int =
    val wild = try (for (y <- List(1); _: Item = make(y)) yield 1).sum catch { case _: MatchError => -1 }
    val parens = try (for (y <- List(1); (l: Item) = make(y)) yield 1).sum catch { case _: MatchError => -1 }
    wild + parens
  def main(args: Array[String]): Unit =
    println(nulls)
    println(widened)
    println(expected)
    println(named)
    println(patterns)
