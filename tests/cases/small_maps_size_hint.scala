// scala-library's `Map.Map1` to `Map4` constructed by name, which zio's `UpdateOrderLinkedMap`
// does, and `mutable.HashSet.sizeHint`.
import scala.collection.mutable

@main def main(): Unit =
  val m1 = new Map.Map1("a", 1)
  val m2 = new Map.Map2("a", 1, "b", 2)
  val m4 = new Map.Map4("a", 1, "b", 2, "c", 3, "d", 4)
  println((m1, m2("b"), m4.size, m4.get("d"), m1.updated("z", 9).toList.sorted, m2 == Map("b" -> 2, "a" -> 1)))
  println(new Map.Map3(1, "x", 2, "y", 3, "z").removed(2))
  val s = new mutable.HashSet[Int]()
  s.sizeHint(10)
  s ++= List(3, 1, 2)
  println(s.toList.sorted)
