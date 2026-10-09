import scala.collection.mutable

case class Key(a: Int, b: String)

@main def run(): Unit =
  val m = Map("one" -> 1, "two" -> 2, "three" -> 3)
  println(m)
  println(m("two"))
  println(m.get("four"))
  println(m.getOrElse("four", 4))
  println(m.contains("one"))
  println(m.size)
  val m2 = m + ("four" -> 4)
  println(m2.size)
  println(m.size)
  println((m - "one").keys.toList)
  println(m.updated("one", 100)("one"))
  println(m.map((k, v) => (k.toUpperCase, v * 10)))
  println(m.filter((_, v) => v > 1))
  println(m.keys.toList.sorted)
  println(m.values.toList.sum)
  println(m.toList.sortBy(_._2))
  for (k, v) <- m do println(s"$k -> $v")
  println(m == Map("one" -> 1, "two" -> 2, "three" -> 3))
  println(m == m2)
  println(Map.empty[Int, Int].isEmpty)

  val byKey = Map(Key(1, "a") -> "first", Key(2, "b") -> "second")
  println(byKey(Key(1, "a")))
  println(byKey.get(Key(3, "c")))
  val tupleKeys = Map((1, 2) -> "x")
  println(tupleKeys.contains((1, 2)))

  val words = List("apple", "avocado", "banana", "blueberry", "cherry")
  val grouped = words.groupBy(_.head)
  println(grouped('a'))
  println(grouped('b'))
  println(grouped.keys.toList.sorted)
  println(List(1 -> "a", 2 -> "b").toMap)
  val counts = words.foldLeft(Map.empty[Int, Int]): (acc, w) =>
    acc.updated(w.length, acc.getOrElse(w.length, 0) + 1)
  println(counts.toList.sorted)

  val s = Set(1, 2, 3)
  println(s)
  println(s.contains(2))
  println(s + 4)
  println(s + 2)
  println(s - 1)
  println(s.map(_ * 2))
  println(s.intersect(Set(2, 3, 4)))
  println(s.union(Set(9)).size)
  println(s.diff(Set(1)))
  println(s == Set(3, 2, 1))
  println(List(1, 1, 2).toSet.size)
  println(Set(Key(1, "a")).contains(Key(1, "a")))

  val buf = mutable.ArrayBuffer[Int]()
  buf += 1
  buf += 2
  buf.append(3)
  buf(0) = 10
  println(buf)
  println(buf.length)
  println(buf.map(_ + 1))
  println(buf.remove(1))
  println(buf.toList)
  buf.insert(0, 99)
  println(buf.sum)
  buf.clear()
  println(buf.isEmpty)

  val mm = mutable.Map[String, Int]()
  mm("a") = 1
  mm("b") = 2
  mm += ("c" -> 3)
  mm("a") = mm("a") + 10
  println(mm("a"))
  println(mm.getOrElseUpdate("d", 4))
  println(mm.getOrElseUpdate("d", 5))
  println(mm.remove("b"))
  println(mm.contains("b"))
  println(mm.keys.toList.sorted)
  println(mm.size)
  val hm = mutable.HashMap.empty[Int, String]
  hm.put(1, "x")
  println(hm.get(1))

  val ms = mutable.Set[String]()
  ms += "x"
  println(ms.add("y"))
  println(ms.add("x"))
  println(ms.contains("x"))
  println(ms.size)
