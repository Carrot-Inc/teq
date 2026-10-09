def show(label: String, values: Any*): Unit = println(label + ": " + values.mkString(" | "))

final case class Day(n: Int):
  def next: Day = Day(n + 1)
  def weekday: Int = n % 7

def keyed[A](as: IterableOnce[A])(key: A => String): String =
  val out = new StringBuilder
  as.iterator.zipWithIndex.foreach((a, i) => out.append(i.toString + "=" + key(a) + ";"))
  out.toString

def isIn[T](v: T)(candidates: T*): Boolean = candidates.iterator.contains(v)

@main def main(): Unit =
  val days = Iterator.iterate(Day(3))(_.next).dropWhile(_.weekday != 0).takeWhile(_.n <= 30)
  show("iterate", days.zipWithIndex.map((d, i) => d.n.toString + "@" + i.toString).toList)
  show("from", Iterator.from(1).map(_ * 2).filter(_ % 3 == 0).take(4).toList)
  show("from step", Iterator.from(10, -3).take(3).toVector, Iterator.from(0).drop(5).next())
  show("continually", Iterator.continually("x").take(3).mkString, Iterator.fill(2)("ab").toList, Iterator.tabulate(3)(_ * 3).toList)
  show("range", Iterator.range(0, 3).toList, Iterator.range(5, 0, -2).toList, Iterator.single(1).toList, Iterator.empty[Int].toList)
  show("unfold", Iterator.unfold(1)(s => if s > 20 then None else Some((s.toString, s * 2))).toList)
  show("apply", Iterator(1, 2, 3).map(_ + 1).toList, Iterator(1, 2, 3).size, Iterator("a").hasNext)
  val it = List(1, 2, 3).iterator
  show("next", it.next(), it.hasNext, it.next(), it.next(), it.hasNext, it.nextOption())
  show("flatMap", Iterator(1, 2, 3).flatMap(x => List.fill(x)(x)).toList, Iterator(1, 2).flatMap(x => Option.when(x > 1)(x)).toList)
  show("lazy flatMap", Iterator.from(1).flatMap(x => Iterator(x, -x)).take(5).toList)
  show("zip", Iterator(1, 2, 3).zip(Iterator.from(10)).toList, Iterator.from(0).zip(List("a", "b")).toList)
  show("concat", (Iterator(1) ++ Iterator(2, 3) ++ List(4)).toList, Iterator(1).concat(Some(2)).toList)
  show("slice", Iterator.from(0).slice(3, 6).toList, Iterator(1, 2, 3).drop(5).toList, Iterator(1, 2, 3).take(0).toList)
  show("filterNot", Iterator(1, 2, 3, 4).filterNot(_ % 2 == 0).toList, Iterator(1, 2, 3).withFilter(_ > 1).map(_ * 2).toList)
  show("consumers", Iterator(3, 1, 2).max, Iterator(3, 1, 2).sum, Iterator(1, 2).foldLeft(10)(_ - _), Iterator(1, 2, 3).find(_ > 1), Iterator(1, 2).exists(_ > 5))
  show("more consumers", Iterator(1, 2, 3).contains(2), Iterator(5, 6, 7).indexOf(7), Iterator(5, 6).indexWhere(_ > 9), Iterator(1, 2, 3).forall(_ > 0), Iterator(1, 2).count(_ > 1))
  show("conversions", Iterator(1, 2).toVector, Iterator(1, 1).toSet, Iterator(1 -> "a").toMap, Iterator(2, 1).toSeq, Iterator("a", "b").mkString("[", ",", "]"))
  show("strict fallback", Iterator(1, 2, 3, 4).grouped(3).toList, Iterator(3, 1, 2).toList.sorted, Iterator(1, 2, 3).scanLeft(0)(_ + _).toList)
  show("sliding", Iterator(1, 2, 3).sliding(2).toList, Iterator(1, 2, 3).zipWithIndex.toList)
  show("collection iterators", Vector(1, 2).iterator.map(_ + 1).toList, Set(1).iterator.toList, Map(1 -> 2).iterator.toList, Some(1).iterator.toList, (1 to 3).iterator.toList)
  show("array iterator", Array(1, 2, 3).iterator.drop(1).toList)
  show("list iterator is lazy", List(1, 2, 3).iterator.takeWhile(_ < 2).toList, List(1, 2, 3).iterator.map(_ * 2).next())
  show("keyed", keyed(List("a", "b"))(_.toUpperCase), keyed(Some(1))(_.toString), keyed(Iterator(1.5))(_.toString), keyed(Set(true))(_.toString))
  show("isIn", isIn(1)(1, 2), isIn("z")("a"), isIn(1)())
  show("tapEach", Iterator(1, 2).tapEach(x => println("saw " + x.toString)).map(_ + 1).toList)
  show("toString", Iterator(1).toString)
  var pulled = 0
  val counting = Iterator.continually({ pulled += 1; pulled })
  val firstBig = counting.map(_ * 2).filter(_ > 4).take(2).toList
  show("pulls", firstBig, pulled)
  val tw = Iterator.from(1).takeWhile(_ < 3)
  show("takeWhile", tw.hasNext, tw.next(), tw.next(), tw.hasNext)
