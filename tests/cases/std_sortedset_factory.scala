// A collection built with evidence of its elements through its companion: `to(SortedSet)`,
// scala-library's EvidenceIterableFactory converted to a Factory over the Ordering.
import scala.collection.immutable.SortedSet
@main def main(): Unit =
  val s = List("b", "a", "c", "a").to(SortedSet)
  println(s)
  println(Vector(3, 1, 2).to(SortedSet).toList)
