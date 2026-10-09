// Maps of two to four entries and the mutable hash maps iterate over their own entries; under
// `--std=scala-library` their iterators are classes nested in the map classes.
package smallmapiterators

import scala.collection.mutable

@main def main(): Unit =
  val names = Array("Red", "Green", "Blue")
  val byName = names.zipWithIndex.toMap
  println(byName)
  println(byName.find(_._1 == "Green"))
  println(List("a" -> 1, "b" -> 2).toMap.toList)
  println(Map(1 -> "a", 2 -> "b", 3 -> "c", 4 -> "d").values.toList)
  println(Map(1 -> "a", 2 -> "b").iterator.map(_._2).mkString)
  val h = mutable.HashMap("x" -> 1, "y" -> 2, "z" -> 3)
  println(h.toList.sorted)
  println(h.values.sum)
  val l = mutable.LinkedHashMap("p" -> 1, "q" -> 2)
  println(l.toList)
