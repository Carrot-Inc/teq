// The rules an inline expansion keeps whatever its mechanism, which the expansion by substitution
// holds to: reduction on the static type, a constant false guard giving way
// to the next case, only the selected branch's deferred operations run, ordinary arguments evaluated
// once in order and by-name and inline ones on each use or never, defaults and repeated parameters
// as ordinary arguments, the receiver evaluated before the arguments. scalac 3.8.4 and teq agree.
import scala.compiletime.error

class Counter:
  var n = 0
  def next(): Int = { n += 1; n }

object Rules:
  // Reduction on the static type of the scrutinee, never on its value.
  inline def kind(x: Any): String = inline x match
    case _: Int => "int"
    case _: String => "string"
    case _ => "other"
  // A matching case whose constant guard is false gives way to the next.
  inline def sign(inline n: Int): String = inline n match
    case k if k > 0 => "positive"
    case k if k < 0 => "negative"
    case _ => "zero"
  // Only the selected branch's deferred operations run.
  inline def choose(inline b: Boolean): Int = inline if b then 1 else error("dead")
  // An ordinary argument is evaluated once, before the body; a by-name or inline one on each use.
  inline def twice(x: Int): Int = x + x
  inline def twiceByName(x: => Int): Int = x + x
  inline def twiceInline(inline x: Int): Int = x + x
  // An unused by-name or inline argument is not evaluated; an unused ordinary one is.
  inline def ignore(x: Int, y: => Int, inline z: Int): Int = 0
  // Defaults and repeated parameters bind as ordinary arguments.
  inline def withDefault(x: Int, y: Int = 10): Int = x + y
  inline def sum(xs: Int*): Int = xs.sum

class Box(val v: Int):
  inline def plus(x: Int): Int = v + x

extension (b: Box)
  inline def times(x: Int): Int = b.v * x

object Main:
  def trace(s: String, v: Int): Int = { println(s); v }
  def box(s: String, v: Int): Box = { println(s); Box(v) }
  def main(args: Array[String]): Unit =
    val any: Any = 1
    println(Rules.kind(1) + " " + Rules.kind("s") + " " + Rules.kind(any))
    println(Rules.sign(3) + " " + Rules.sign(-2) + " " + Rules.sign(0))
    println(Rules.choose(true))
    val c1 = Counter(); println((Rules.twice(c1.next()), c1.n))
    val c2 = Counter(); println((Rules.twiceByName(c2.next()), c2.n))
    val c3 = Counter(); println((Rules.twiceInline(c3.next()), c3.n))
    val c4 = Counter(); println((Rules.ignore(c4.next(), c4.next(), c4.next()), c4.n))
    println(s"${Rules.withDefault(1)} ${Rules.withDefault(1, 2)}")
    println(Rules.sum(trace("a", 1), trace("b", 2)))
    println(box("receiver", 3).plus(trace("argument", 4)))
    println(box("extension receiver", 5).times(trace("argument", 6)))
