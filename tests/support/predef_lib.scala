// The jar side of tests/classpath/js/jar_predef_bodies.scala: bodies scalac compiles through
// scala-library's Predef, which a program's source would not name.
package predeflib

object Arrays:
  // `copyArrayToImmutableIndexedSeq`: an array where an immutable sequence is expected.
  def asSeq(xs: Array[String]): Seq[String] = xs
  def indexed(xs: Array[Int]): IndexedSeq[Int] = xs

object Pairs:
  // `==` of two tuples, which a body selects as a method of `Any`.
  def same(a: String, b: Int, c: String, d: Int): Boolean = (a, b) == (c, d)
  def differ(a: String, b: Int, c: String, d: Int): Boolean = (a, b) != (c, d)

final case class Trace[A](v: A):
  def unary_!(using ev: A <:< Boolean): Trace[Boolean] = Trace(!ev(v))

object Traces:
  // A prefix operator with an evidence clause, selected and applied as zio-test's `TestArrow`
  // negates a trace.
  def negated(t: Trace[Boolean]): Trace[Boolean] = t.unary_!(using <:<.refl[Boolean])

object Unzips:
  // `unzip` and `unzip3`, which a body applies to their type arguments and `Predef.$conforms`.
  def split(xs: List[(Int, String)]): (List[Int], List[String]) = xs.unzip
  def split3(xs: Vector[(Int, String, Boolean)]): (Vector[Int], Vector[String], Vector[Boolean]) = xs.unzip3

class Multi(val parts: String*)

object Multis:
  // A repeated parameter read as a value, which scalac ascribes with its repeated type (fansi's
  // `Attrs.toSeq`).
  def partsOf(m: Any): Seq[String] = m match
    case m: Multi => m.parts
    case s: String => Seq(s)

object ArrayFors:
  // A for comprehension over an array with a guard: `ArrayOps.withFilter`, then its `map` with
  // the `ClassTag` of the result (utest's `BaseRunner.tasks`).
  def evens(xs: Array[Int]): Array[String] = for x <- xs if x % 2 == 0 yield "#" + x

object ArrayScans:
  // `indexWhere` and `lastIndexWhere` with their default bounds (munit's `StackTraces`).
  def firstAndLast(xs: Array[Int]): (Int, Int) = (xs.indexWhere(_ > 1), xs.lastIndexWhere(_ > 1))
