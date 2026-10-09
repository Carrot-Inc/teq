// A mutable map of sets with MultiMap mixed in, folded from pairs and read back as an
// immutable map through mapValues, as izumi-reflect builds its inheritance databases.
import scala.collection.mutable

object Main:
  def main(args: Array[String]): Unit =
    val pairs = List("a" -> 1, "b" -> 2, "a" -> 3, "a" -> 1)
    val mm = pairs.foldLeft(new mutable.HashMap[String, mutable.Set[Int]] with mutable.MultiMap[String, Int]) {
      (map, pair) => map.addBinding(pair._1, pair._2)
    }
    println(mm.entryExists("a", _ > 2))
    mm.removeBinding("b", 2)
    val frozen = mm.mapValues(_.toSet).toMap
    println(frozen.toList.sortBy(_._1).map((k, v) => s"$k=${v.toList.sorted}"))
