// scala-library's `Map.Map1` to `Map4` constructed as classes, and a mutable `HashSet`'s
// `sizeHint`, as zio's `UpdateOrderLinkedMap.single` and `ZEnvironment.prune` use them.
import scala.collection.mutable

object Main:
  def main(args: Array[String]): Unit =
    val m1: Map[String, Int] = new Map.Map1("a", 1)
    println(m1)
    println(m1.updated("b", 2))
    println(new Map.Map2("a", 1, "b", 2).get("b"))
    println(new Map.Map3("a", 1, "b", 2, "c", 3).removed("a").size)
    println(new Map.Map4("a", 1, "b", 2, "c", 3, "d", 4).keys.toList.sorted)
    val s = mutable.HashSet(1, 2)
    s.sizeHint(10)
    s += 3
    println(s.toList.sorted)
