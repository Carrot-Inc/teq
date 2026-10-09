// A ListMap partitioned into two ListMaps, keeping the insertion order.
import scala.collection.immutable.ListMap
@main def main(): Unit =
  val m = ListMap("c" -> 3, "a" -> 1, "b" -> 2)
  val (odd, even): (ListMap[String, Int], ListMap[String, Int]) = m.partition(_._2 % 2 == 1)
  println(odd)
  println(even)
