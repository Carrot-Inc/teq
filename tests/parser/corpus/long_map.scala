// scala-library's `immutable.LongMap`, as zio's `Scope` keeps its finalizers: keys in unsigned
// order (0 up to Long.MaxValue, then Long.MinValue up to -1), `updated`, `-`, `getOrElse`,
// `firstKey`, `values`, printed.
import scala.collection.immutable.LongMap

object Main:
  def main(args: Array[String]): Unit =
    var m = LongMap.empty[String]
    println(m.isEmpty)
    m = m.updated(-1L, "a").updated(-2L, "b").updated(-3L, "c")
    println(m)
    println(m.firstKey)
    println(m.lastKey)
    println(m.values.toList)
    println(m(-2L))
    println(m.getOrElse(7L, "none"))
    val n = m - (-2L)
    println(n)
    println(n.size)
    val mixed = LongMap(5L -> "five", -5L -> "minus", 0L -> "zero", Long.MaxValue -> "max", Long.MinValue -> "min")
    println(mixed.keys.toList)
    println(mixed.get(0L))
    println(mixed.contains(6L))
    println((mixed + (3L -> "three")).keys.toList)
    println(mixed.map { case (k, v) => (k + 1, v.length) })
