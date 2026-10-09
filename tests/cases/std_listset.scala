// The lean std's insertion-ordered ListSet, as scala-library's iterates and prints it.
import scala.collection.immutable.ListSet
@main def main(): Unit =
  val s = ListSet(3, 1, 2, 1)
  println(s)
  println(s + 5 + 3)
  println((s - 1).toVector)
  println(s.filterNot(_ > 2).nonEmpty)
  println(ListSet.empty[String] ++ List("b", "a", "b"))
  println(s.map(_ * 2).contains(6))
  println(ListSet.from(List(4, 4, 2)).size)
