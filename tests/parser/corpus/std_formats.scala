import scala.collection.immutable.ListMap
@main def main(): Unit =
  println(s"${(1 to 5).take(2)} ${(1 to 5).drop(2)} ${(1 to 5).slice(1, 3)} ${(1 to 5).takeWhile(_ < 3)} ${(1 to 5).dropWhile(_ < 3)}")
  println(s"${(1 to 5).takeRight(2)} ${(1 to 5).dropRight(2)} ${(1 to 5).tail} ${(1 to 5).init} ${(1 to 5).take(0)} ${(1 to 5).drop(9)}")
  println(s"${(1 until 5).take(2)} ${(1 to 10 by 3).take(2)} ${(1 to 10 by 3).drop(1)} ${(1 to 5).reverse} ${(1 to 5).filter(_ > 2)} ${(1 to 5).splitAt(2)} ${(1 to 5).span(_ < 3)}")
  println(s"${(1 to 3).map(_ * 2)} ${(1 to 3).sum} ${(1 to 5).take(2).toList} ${(1 to 5).drop(2).sum} ${(5 to 1 by -1).take(2)} ${(1 to 5).slice(1, 3).length}")
  println(s"${1L to 3L} ${1L until 3L} ${1L to 10L by 3L} ${3L to 1L} ${(1L to 3L).map(_ * 2)} ${(1L to 3L).length} ${(1L to 3L).sum} ${(1L to 3L).toList} ${(1L to 3L).contains(2L)} ${(10L to 1L by -4L).toList}")
  println(s"${'a' to 'c'} ${'a' until 'c'} ${('a' to 'c').toList} ${('a' to 'c').map(_.toUpper)} ${('a' to 'c').length} ${('a' to 'e' by 2).toList} ${('a' to 'c').mkString}")
  var total = 0L
  for i <- 0L until 6L by 2L do total += i
  println(total)
  val s: Seq[Int] = Array(1, 2, 3, 4).toIndexedSeq
  println(s)
  println(Array(1, 2).toSeq)
  println(Array(1, 2).toIndexedSeq.map(_ + 1))
  val empty = ListMap.empty[Int, Int]
  val m0 = ListMap(1 -> 1, 2 -> 2)
  println((m0 - 3) eq m0)
  println(((m0 - 1) - 2) eq empty)
  println((empty - 1) eq empty)
  println((m0 -- List(1, 2)) eq empty)
  println((Map(1 -> 1).filter(_ => false)) eq Map.empty)
  println(s"${((Set(1) - 1) eq Set.empty)} ${((Set(1, 2) -- List(1, 2)) eq Set.empty)} ${((Set.empty[Int] - 1) eq Set.empty)}")
  println(Map.empty[Int, Int] + (1 -> 1))
  println(Set.empty[Int] + 1)
  println(Map.empty[Int, Int] ++ List(1 -> 1, 2 -> 2))
  println(Set.empty[Int] ++ List(1, 2))
  println("""#a
     #b""".stripMargin('#'))
  println("""|a
     |b""".stripMargin)
  println(Iterator(1, 2, 3).grouped(2).toList)
  println(Iterator(1, 2, 3).sliding(2).toList)
