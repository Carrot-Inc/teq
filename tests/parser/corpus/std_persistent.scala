import scala.collection.immutable.ListMap

def show(label: String, values: Any*): Unit = println(label + ": " + values.mkString(" | "))

final case class Key(id: Int, tag: String)

def entriesByKey[V](m: Map[Int, V]): List[(Int, V)] = m.toList.sortBy(_._1)

@main def main(): Unit =
  val base = ListMap(1 -> "a", 2 -> "b", 3 -> "c")
  val added = base + (4 -> "d")
  val updated = added.updated(2, "B")
  val removed = updated - 1
  show("versions", base.toList, added.toList, updated.toList, removed.toList)
  show("old versions after use", base.get(4), added.get(2), updated.get(1), removed.get(1), base.size, removed.size)

  val readded = removed + (1 -> "again")
  show("removed and added again", readded.toList, removed.toList, updated.toList)
  show("order of older versions", base.keys.toList, added.keys.toList, updated.keys.toList, removed.keys.toList, readded.keys.toList)

  val left = base + (10 -> "left")
  val right = base + (20 -> "right")
  val further = left + (11 -> "left too")
  show("branches", left.toList, right.toList, further.toList, base.toList)
  show("branch removal", (left - 2).toList, (right - 2).toList, (base - 2).toList, base.toList)

  val chain = List.range(0, 50).foldLeft(ListMap.empty[Int, Int])((m, i) => m + (i -> i * i))
  val pruned = List.range(0, 45).foldLeft(chain)((m, i) => m - i)
  val regrown = pruned + (3 -> -3) + (100 -> 100)
  show("long chain", chain.size, chain.head, chain.last, pruned.toList, regrown.toList)
  show("chain intact", chain.keys.toList == List.range(0, 50), chain(7), chain.get(49), chain.contains(50))

  var snapshots = List.empty[Map[Int, Int]]
  var current = Map.empty[Int, Int]
  List.range(0, 6).foreach: i =>
    current = if i % 3 == 2 then current - (i - 1) else current.updated(i, i * 10)
    snapshots = current :: snapshots
  show("snapshots", snapshots.reverse.map(entriesByKey))

  val counts = "the quick brown fox jumps over the lazy dog the end".split(" ").foldLeft(Map.empty[String, Int]): (m, w) =>
    m.updated(w, m.getOrElse(w, 0) + 1)
  show("counts", counts("the"), counts.get("fox"), counts.size, counts.toList.sortBy(_._1).take(3))

  var growing = Map(1 -> 1)
  growing.foreach((k, v) => growing = growing + (k + 1 -> v) + (k -> 0))
  show("update while iterating", entriesByKey(growing))
  val source = Map(1 -> "x", 2 -> "y")
  show("self concat", entriesByKey(source ++ source), entriesByKey(source ++ source.map((k, v) => (k + 1, v))), entriesByKey(source -- source.keys), entriesByKey(source))

  val m1 = Map(1 -> "a") + (2 -> "b")
  val m2 = Map(2 -> "b") + (1 -> "a")
  val m3 = m1 - 2
  show("equality", m1 == m2, m1.hashCode == m2.hashCode, m3 == Map(1 -> "a"), m1 == m3, (m3 + (2 -> "b")) == m1, m1 == m2.updated(1, "z"))

  val keyed = List.range(0, 20).foldLeft(Map.empty[Key, Int])((m, i) => m + (Key(i % 10, "k") -> i))
  val fewer = keyed - Key(3, "k") - Key(99, "k")
  show("structural keys", keyed.size, keyed(Key(3, "k")), fewer.get(Key(3, "k")), fewer.size, keyed.get(Key(3, "k")), (fewer + (Key(3, "k") -> 0)).size)

  val s0 = Set(1, 2, 3)
  val s1 = s0 + 4
  val s2 = s1 - 2
  val s3 = s2 + 2
  show("sets", s0.toList.sorted, s1.toList.sorted, s2.toList.sorted, s3.toList.sorted, s0 == (s2 - 4 + 2), s0.contains(4), s2.contains(2), s1.contains(2))
  val big = List.range(0, 2000).foldLeft(Set.empty[Int])(_ + _)
  val evens = List.range(0, 2000).filter(_ % 2 == 1).foldLeft(big)(_ - _)
  show("set chain", big.size, evens.size, evens.contains(1000), evens.contains(1001), big.contains(1001), (evens ++ List(1, 3)).size, (big -- evens).size, big.size)
  show("set algebra", (s1 | s3).toList.sorted, (s1 & s2).toList.sorted, (s1 &~ s2).toList.sorted, s2.subsetOf(s1), s1.subsetOf(s2))

  val v0 = Vector(1, 2, 3)
  val v1 = v0 :+ 4
  val v2 = v0 :+ 5
  val v3 = v1 :+ 6
  val widened: Vector[Any] = v0 :+ "seven"
  show("vector branches", v0, v1, v2, v3, widened, v0.length, v1.last, v2.last, v0.iterator.toList, v1.toList, v0.toArray.length)
  val it = v3.iterator
  val v4 = v3 :+ 7
  show("vector iterator", it.toList, v4, v3 == Vector(1, 2, 3, 4, 6), v3.appended(8), v3.updated(0, 0), v3.map(_ * 2), v3.reverse, v3.drop(3))
  val grown = List.range(0, 1000).foldLeft(Vector.empty[Int])(_ :+ _)
  val array = Array(1, 2)
  val longer = array :+ 3
  show("vector chain", grown.length, grown.sum, grown.take(3), (grown :+ -1).last, grown.last, array.toList, longer.toList)
