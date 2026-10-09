// scala-library's `immutable.HashMap` as zio's `UpdateOrderLinkedMap` keeps an environment in
// it: built with `newBuilder`, updated, removed and read, a `Map` that prints by its own name.
import scala.collection.immutable.HashMap

object Main:
  def main(args: Array[String]): Unit =
    val b = HashMap.newBuilder[String, Int]
    b += ("a" -> 1)
    b += ("b" -> 2)
    val m: HashMap[String, Int] = b.result()
    val n: HashMap[String, Int] = m.updated("c", 3) - "a"
    println(n.get("c"))
    println(n.getOrElse("a", 0))
    println(n.size)
    println(n.contains("b"))
    println(HashMap("x" -> 9))
    val asMap: Map[String, Int] = n
    println(asMap("b"))
    println(HashMap.empty[Int, Int].isEmpty)
    println(n == Map("b" -> 2, "c" -> 3))
