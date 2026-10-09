package probes.stdwider

// A std member whose scala-library result is wider than the lean std's (`Map.values` an
// `Iterable`, `sizeIs` an `IterableOps.SizeCompareOps`):
// written as the receiver of a member selected by name whose scala-library result on
// the wider class is the lean std's (`toList`, `size`) or of a comparison (`compared`), withheld
// where a binding of the lean type would hold it (scalac's read fails on `Found: Iterable[Int],
// Required: List[Int]`), where its consumer is selected at a signature, which names the lean
// receiver's class (`sum`, `map`), and where a member selected by name returns the wider
// collection again (`tail`, scala-library's `C`).
object StdWider:
  def listed(m: Map[String, Int]): List[Int] = m.values.toList
  def compared(xs: List[Int]): Boolean = xs.sizeIs > 1
  def summed(xs: Map[String, Int]): Int = xs.values.sum
  def mapped(m: Map[String, Int]): Iterable[Int] = m.values.map(_ + 1)
  def stored(xs: Map[String, Int]): Int =
    val vs = xs.values
    vs.sum
  def storedSize(xs: List[Int]): Boolean =
    val n = xs.sizeIs
    n > 1
  def sized(m: Map[String, Int]): Int = m.values.size
  def storedTail(m: Map[String, Int]): Int =
    val xs = m.values.tail
    xs.sum
  def mappedTail(m: Map[String, Int]): Iterable[Int] = m.values.tail.map(_ + 1)

// `case Array()` on a scrutinee it tests, which scalac tests as `Array[T$1]`: withheld, an
// `Array[Any]` test would let no primitive array through.
object ArrayOnAny:
  def empty(x: Any): Boolean = x match
    case Array() => true
    case _ => false
