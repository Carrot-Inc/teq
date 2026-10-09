//> using platform js
def show(label: String, values: Any*): Unit = println(label + ": " + values.mkString(" | "))

final case class Item(id: Int, group: String, rank: Option[Int])

@main def main(): Unit =
  show("int", List("+5", " 5", "5 ", "-0", "2147483647", "2147483648", "-2147483648", "1.0", "0x1F", "", "-", "00012", "1_000").map(_.toIntOption))
  show("long", List("9223372036854775807", "9223372036854775808", "-9223372036854775808", "+7", "7L", " 7").map(_.toLongOption))
  show("double", List("1.", ".5", "1e", "1d", " 1 ", "Infinity", "-Infinity", "1_0", "1e400", "+.5e-2", "1f", "NaN", "nan", "1,5", "--1").map(s => s.toDoubleOption.map(d => d.toString)))
  show("bool", List("TRUE", "False", "yes", "", "true ").map(_.toBooleanOption))
  show("split", "a,b,,c,,".split(",").toList, ",a,,b".split(",").toList, "abc".split("").toList, "a1b22c".split("\\d+").toList, "a|b".split("\\|").toList, "a b  c".split(" ").toList, "x".split(",").toList, ",".split(",").toList, "a,b,c".split(",", 2).toList, "a,b,,".split(",", -1).toList)
  show("replace", "a.b.c".replace(".", "-"), "a.b.c".replaceAll(".", "-"), "a.b.c".replaceAll("\\.", "\\$"), "aaa".replaceFirst("a", "b"), "x1y22".replaceAll("(\\d)", "<$1>"), "Hello World".replaceAll("(?i)[aeiou]", "*"), "tab\there".replaceAll("\\s", "_"))
  val items = List(Item(3, "b", Some(2)), Item(1, "a", None), Item(2, "b", Some(1)), Item(4, "a", Some(1)), Item(5, "c", None))
  show("sortBy option", items.sortBy(_.rank).map(_.id), items.sortBy(i => (i.group, i.rank)).map(_.id), items.sortBy(i => (i.rank.isEmpty, -i.id)).map(_.id))
  show("groupBy", items.groupBy(_.group).toList.sortBy(_._1).map((k, v) => (k, v.map(_.id))), items.groupMap(_.group)(_.id).toList.sortBy(_._1), items.groupMapReduce(_.group)(_ => 1)(_ + _).toList.sortBy(_._1))
  show("minmax", items.maxBy(_.id).id, items.minBy(_.rank).id, items.map(_.id).max, items.flatMap(_.rank).min, items.map(_.rank).max, items.map(_.group).distinct, items.map(_.id).sum, items.map(_.id.toDouble).sum == 15.0, items.map(_.id).product)
  show("zip", items.map(_.id).zip(items.map(_.group)).toMap.toList.sortBy(_._1).take(2), items.zipWithIndex.map((i, n) => i.id * n), items.map(_.id).zipAll(List("x"), 0, "-"), List(1, 2, 3).zip(List("a", "b")).unzip)
  show("sliding", List(1, 2, 3, 4, 5).sliding(2).toList, List(1, 2, 3, 4, 5).sliding(3, 2).toList, List(1, 2, 3, 4, 5).grouped(2).toList, List(1, 2, 3).tails.toList, List(1, 2, 3).inits.toList)
  show("folds", List(1, 2, 3).foldRight(List.empty[Int])(_ :: _), List(1, 2, 3).reduceLeft(_ - _), List(1, 2, 3).reduceRight(_ - _), List(1, 2, 3).scanLeft(0)(_ + _), List(1, 2, 3).scanRight(0)(_ + _), List.empty[Int].reduceOption(_ + _), List(1, 2, 3).foldLeft("")(_ + _))
  show("misc", List(1, 2, 3).mkString("[", ", ", "]"), List(1, 2, 3).indices.toList, List(List(1, 2), List(3, 4)).transpose, Vector(List(1, 2, 3), List(4, 5, 6)).transpose, List.empty[List[Int]].transpose, List(1, 2, 3).reduceRightOption(_ - _), Vector(1, 2).tails.toList, Vector(1, 2).scanRight(10)(_ + _))
