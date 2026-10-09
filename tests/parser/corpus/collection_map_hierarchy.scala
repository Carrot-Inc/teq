// `collection.Map` above the immutable `Map` and the mutable `HashMap`, as scala-library has it:
// either passes where a `collection.Map` is asked for, and its reads work on both.
import scala.collection.{immutable, mutable}

object Main:
  def describe[K, V](m: collection.Map[K, V], k: K): String =
    s"${m.size} ${m.contains(k)} ${m.get(k)} ${m.getOrElse(k, "none")}"
  def total(m: collection.Map[String, Int]): Int = m.foldLeft(0)((acc, e) => acc + e._2)
  def main(args: Array[String]): Unit =
    val a: immutable.Map[String, Int] = immutable.Map("a" -> 1, "b" -> 2)
    val b = mutable.HashMap("b" -> 2)
    b("c") = 3
    println(describe(a, "a"))
    println(describe(b, "a"))
    println(describe(immutable.HashMap("x" -> 9), "x"))
    println(total(a) + total(b))
    val m: collection.Map[String, Int] = b
    println(m("c"))
    println(collection.Map("k" -> 1).get("k"))
