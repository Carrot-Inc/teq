// A map or set made in bulk (Map.from, toMap, toSet, groupBy, ++) is updated, probed and
// reduced afterwards: the index the bulk build made serves every later operation, under the
// interpreter's natives, without them (TEQ_NO_NATIVES=1) and without the trie's ones alone
// (TEQ_NO_NATIVES=trie), which the suite's run covers in the first mode and the package's
// report in the others.
@main def main(): Unit =
  val m = Map.from((0 until 9).map(i => (i, i)))
  val m2 = m.updated(9, 9)
  println("" + m2.get(9) + " " + m2.size + " " + (m2 - 4).size + " " + (m2 - 4).contains(4) + " " + m2.contains(8))
  val big = (0 until 40).map(i => ("k" + i, i)).toMap
  val grown = big.updated("k40", 40).updated("k3", -3)
  println("" + grown.size + " " + grown("k3") + " " + grown("k40") + " " + (grown -- (0 until 30).map(i => "k" + i)).size + " " + (grown -- (0 until 30).map(i => "k" + i)).get("k35"))
  val s = Set.from(0 until 40) + 50
  println("" + s.size + " " + s.contains(50) + " " + (s - 7).size + " " + (s - 7).contains(7) + " " + (s ++ (100 until 200)).size)
  val g = (0 until 100).groupBy(_ % 7)
  val g2 = g.updated(7, List(-1)).removed(0)
  println("" + g2.size + " " + g2(7) + " " + g2.get(0) + " " + g2(3).length)
  val merged = m ++ big.map((k, v) => (v + 100, v))
  println("" + merged.size + " " + merged.get(139) + " " + merged.removed(139).size + " " + merged.keys.toList.sorted.take(3))
