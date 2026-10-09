// A map view mapped to pairs with the two type arguments of scala-library's MapOps.map, and to
// single values, and concatenated with entries of a wider value type.
@main def main(): Unit =
  val m = List("a" -> 1, "bb" -> 2, "a" -> 3).groupBy(_._1)
  println(m.view.map[String, Int]((k, v) => (k * 2, v.map(_._2).sum)).toList.sortBy(_._2))
  println(m.view.map(e => e._1.length).toList.sorted)
  val a = Map("x" -> 1).view
  val b = Map("y" -> 2).view
  println(a.++[Int](b).map(_._2).toList)
  println((a ++ b).filter(_._2 > 1).toList)
