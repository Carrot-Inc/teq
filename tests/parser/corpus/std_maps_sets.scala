import scala.collection.immutable.{ListMap, SortedSet}
import scala.collection.mutable

final case class Id(value: Int)

given Ordering[Id] = Ordering.by(_.value)

def show(label: String, values: Any*): Unit = println(label + ": " + values.mkString(" | "))

@main def main(): Unit =
  val m = Map("a" -> 1, "b" -> 2, "c" -> 3)
  show("find", (m.find(_._2 > 1), m.find(_._2 > 5)))
  show("exists", (m.exists(_._2 == 3), m.forall(_._2 > 1), m.count(_._2 > 1)))
  show("filter", m.filter(_._2 != 2))
  show("filterNot", m.filterNot(_._2 == 2))
  show("partition", m.partition(_._2 > 1))
  show("keys", (m.keys.toList, m.values.toList, m.keySet))
  show("values sum", (m.values.sum, m.values.max, m.keys.mkString("|")))
  show("iterators", (m.keysIterator.toList, m.valuesIterator.map(_ * 2).toList))
  show("removedAll", (m.removedAll(List("a", "z")), m -- Set("b", "c")))
  show("updatedWith", (m.updatedWith("a")(_.map(_ + 10)), m.updatedWith("a")(_ => None), m.updatedWith("z")(_ => Some(0))))
  show("updatedWith none", m.updatedWith("z")(v => v))
  show("++ list", m ++ List("d" -> 4, "a" -> 0))
  show("++ option", m ++ Some("e" -> 5))
  show("concat", m.concat(Map("c" -> 30)))
  show("view mapValues", m.view.mapValues(_ * 2).toMap)
  show("view filterKeys", m.view.filterKeys(_ != "a").toMap)
  show("transform", m.transform((k, v) => k + v.toString))
  show("map pairs", m.map((k, v) => v -> k))
  show("flatMap pairs", m.flatMap((k, v) => List(k -> v, (k + k) -> v * 2)).size)
  show("toList", m.toList)
  show("toList map", m.toList.map((k, v) => k * v))
  show("foldLeft", m.foldLeft(0)((acc, e) => acc + e._2))
  show("minBy", (m.minBy(e => -e._2), m.maxBy(_._2), m.head, m.last, m.headOption))
  show("zipWithIndex", m.zipWithIndex.toList)
  show("unzip", (m.unzip._1.toList, m.unzip._2.toList))
  show("groupBy", m.groupBy(_._2 % 2).toList.sortBy(_._1))
  show("take", (m.take(2), m.drop(2), m.tail, m.init))
  show("mkString", (m.mkString(", "), m.mkString("{", "; ", "}"), Map.empty[Int, Int].mkString))
  show("isDefinedAt", (m.isDefinedAt("a"), m.isDefinedAt("q"), m.nonEmpty, m.size))
  show("Map.from", (Map.from(List(1 -> "x")), Map.from(Some(2 -> "y")), List(3 -> "z").toMap, Vector(4 -> "w").toMap))
  show("option toMap", Option(1 -> 2).toMap)
  show("equality", (m == Map("c" -> 3, "b" -> 2, "a" -> 1), m == m.updated("a", 9), m.hashCode == Map("b" -> 2, "a" -> 1, "c" -> 3).hashCode))
  val asIterable: Iterable[(String, Int)] = m
  show("iterable map", asIterable.map(_._2).toList)
  show("iterable filter", asIterable.filter(_._2 > 1).toList)

  val s = Set(3, 1, 2)
  show("set find", (s.find(_ > 1).isDefined, s.find(_ > 5)))
  show("set min max", (s.min, s.max, s.sum, s.maxBy(x => -x)))
  show("set --", (s -- List(1, 9), s ++ List(7), s ++ Some(8), s ++ Set(4)))
  show("set ops", s & Set(1, 2, 9), s | Set(9), s &~ Set(1), s.intersect(Set(3)), s.diff(Set(3)), s.union(Set(0)))
  show("set incl", s.incl(1) == s, s.incl(5), s.excl(3), s.excl(42), s + 4, s - 1)
  show("set map", (s.map(_ % 2), s.flatMap(x => Set(x, x * 10)).size, s.filter(_ > 1), s.filterNot(_ > 1)))
  show("set flatMap option", s.flatMap(x => if x > 1 then Some(x) else None))
  show("set partition", s.partition(_ > 1))
  show("set toList", s.toList, s.toVector, s.toSeq.sorted, s.size, s.head, s.contains(2), s(9))
  show("set subsetOf", (Set(1).subsetOf(s), Set(1, 9).subsetOf(s), s.removedAll(Set(1, 2)), s.concat(List(1, 5))))
  show("set equality", (s == Set(1, 2, 3), s == Set(1, 2), Set(List(1)) == Set(List(1)), s.hashCode == Set(2, 3, 1).hashCode))
  show("set mkString", (s.mkString, s.mkString(","), s.mkString("<", ",", ">")))
  show("set from", (Set.from(List(1, 1, 2)), Set.from(Some(1)), List(1, 1).toSet, Vector("a").toSet, Set.empty[Int]))
  show("set zip", (s.zipWithIndex.toList.sorted, s.exists(_ > 2), s.forall(_ > 2), s.count(_ > 1), s.foldLeft(0)(_ + _)))
  show("set groupBy", s.groupBy(_ % 2).toList.sortBy(_._1))

  val sorted = SortedSet(5, 1, 3, 1)
  show("sorted", sorted)
  show("sorted ops", sorted + 2, sorted + 3, sorted - 3, sorted - 9, sorted.contains(3), sorted.contains(4))
  show("sorted bulk", (sorted ++ List(9, 0, 3), sorted -- List(1, 5), sorted.concat(Set(4))))
  show("sorted consumers", sorted.toList, sorted.head, sorted.last, sorted.min, sorted.max, sorted.size)
  show("sorted keys", (sorted.firstKey, sorted.lastKey, sorted.filter(_ > 1), sorted.take(2), sorted.drop(1)))
  show("sorted partition", sorted.partition(_ > 2))
  show("sorted range", (sorted.range(1, 5), sorted.rangeFrom(3), sorted.rangeUntil(3)))
  show("sorted set ops", (sorted & Set(1, 5, 7), sorted | Set(2), sorted.diff(Set(1)), sorted.subsetOf(Set(1, 3, 5, 7))))
  show("sorted equality", (sorted == Set(1, 3, 5), Set(5, 3, 1) == sorted, sorted == SortedSet(1, 3), sorted.hashCode == Set(1, 3, 5).hashCode))
  show("sorted map", (sorted.map(_ * 2).toList.sorted, sorted.toSet == Set(1, 3, 5), sorted.exists(_ == 3), sorted.find(_ > 1)))
  val ids = SortedSet.from(List(Id(3), Id(1), Id(2)))
  show("sorted ids", (ids, ids.map(_.value).toList.sorted, ids + Id(0), SortedSet.empty[Id] + Id(9)))
  val toggle = (set: SortedSet[Id]) => (id: Id) => if set.contains(id) then set - id else set + id
  show("toggle", (toggle(ids)(Id(1)), toggle(ids)(Id(7))))
  val asSet: Set[Int] = sorted
  show("sorted as set", (asSet.contains(3), asSet + 0, asSet.toList))
  val strings: SortedSet[String] = SortedSet.empty
  show("sorted strings", (strings + "b" + "a" + "c", strings.isEmpty, (strings + "x").nonEmpty))

  val lm = ListMap("z" -> 1, "y" -> 2, "x" -> 3, "w" -> 4, "v" -> 5, "u" -> 6)
  show("listmap order", lm.toList)
  show("listmap update keeps slot", lm.updated("y", 20).toList)
  show("listmap add", (lm + ("a" -> 0)).keys.toList)
  show("listmap remove add", (lm - "z" + ("z" -> 9)).keys.toList)
  show("listmap takeRight", lm.takeRight(2).toList)
  show("listmap from", ListMap.from(List(3 -> "c", 1 -> "a", 2 -> "b", 0 -> "z", 9 -> "n")).values.toList)
  val emptyLm: ListMap[Int, String] = ListMap.empty
  show("listmap empty", (emptyLm.isEmpty, emptyLm.updated(1, "a").toList))

  val buf = mutable.ArrayBuffer(3, 1, 2)
  buf += 5
  buf ++= List(9, 8)
  buf -= 1
  buf.prepend(0)
  buf.insert(1, 7)
  show("buffer", buf, buf.length, buf(1), buf.toList, buf.sorted, buf.map(_ + 1), buf.filter(_ > 3))
  show("buffer ops", buf.indexOf(5), buf.contains(9), buf.head, buf.last, buf.sum, buf.zipWithIndex.take(2), buf.mkString(","))
  buf.remove(0)
  buf.sortInPlace()
  show("buffer sorted", buf)
  buf.filterInPlace(_ % 2 == 1)
  buf.mapInPlace(_ * 10)
  show("buffer in place", (buf, buf.reverse, buf.isEmpty, buf.nonEmpty))
  buf.clear()
  show("buffer cleared", (buf, buf.isEmpty, buf.size))

  val hm = mutable.HashMap.empty[String, Int]
  hm("a") = 1
  hm.update("b", 2)
  hm += ("c" -> 3)
  show("hashmap", hm.get("a"), hm.getOrElse("z", 0), hm.getOrElseUpdate("d", 4), hm.put("a", 10), hm.put("e", 5), hm.size)
  show("hashmap remove", (hm.remove("b"), hm.remove("zz"), hm.contains("b"), hm.keys.toList.sorted, hm.values.toList.sorted))
  show("hashmap consumers", (hm.toList.sorted, hm.exists(_._2 > 9), hm.find(_._1 == "c"), hm.map((k, v) => k -> v * 2).toList.sorted))
  show("hashmap updateWith", (hm.updateWith("c")(_.map(_ + 1)), hm.updateWith("c")(_ => None), hm.updateWith("q")(_ => Some(7)), hm.toMap.toList.sorted))
  hm.filterInPlace((k, v) => v > 4)
  show("hashmap filterInPlace", hm.toList.sorted)
  hm ++= List("x" -> 1)
  hm --= List("a")
  hm.clear()
  show("hashmap cleared", (hm.isEmpty, hm.size))

  val lru = mutable.LinkedHashMap.empty[String, Int]
  lru("one") = 1
  lru("two") = 2
  lru("three") = 3
  lru.remove("one")
  lru("one") = 11
  show("linked", (lru.headOption, lru.toList, lru.size, lru.keys.toList, lru.head._1))

  val seen = mutable.Set[Any]()
  seen += 1
  seen += "a"
  show("mutable set", seen.add(1), seen.add(2), seen.contains("a"), seen.size, seen.remove("a"), seen.remove("zz"), seen(2))
  val hs = mutable.HashSet(1, 2, 3)
  hs ++= List(4, 5)
  hs --= List(1)
  hs.filterInPlace(_ % 2 == 0)
  show("hashset", (hs.toList.sorted, hs.map(_ * 2).toList.sorted, hs.exists(_ > 3), hs.toSet == Set(2, 4)))
