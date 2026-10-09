// The views of a `java.util.TreeMap` against the JDK's: sub-, head- and tail-maps with inclusive
// and exclusive bounds, which write through, refuse an insertion out of their range and treat a
// lookup or a removal out of it as absent; the views of a view and the bounds it refuses; the
// descending map and key set, views too; the key sets, which remove (by `remove`, `clear`,
// `pollFirst` and their iterator) and add nothing; the entry sets' `setValue`; and the bulk
// removals of key, value and entry views, bounded and descending, over duplicate values.
import java.util as ju

object Main:
  def attempt(label: String)(body: => Any): Unit =
    val out =
      try "" + body
      catch
        case e: IllegalArgumentException => s"IllegalArgumentException(${e.getMessage})"
        case e: RuntimeException => e.getClass.getName
    println(s"$label: $out")

  def tens(): ju.TreeMap[Int, String] =
    val m = new ju.TreeMap[Int, String]()
    for i <- 1 to 9 do m.put(i * 10, "v" + i)
    m

  def main(args: Array[String]): Unit =
    val m = tens()
    println(s"${m.subMap(20, 50)} ${m.subMap(20, false, 50, true)} ${m.subMap(25, true, 55, true)}")
    println(s"${m.headMap(40)} ${m.headMap(40, true)} ${m.headMap(5)} ${m.tailMap(70)} ${m.tailMap(70, false)} ${m.tailMap(95)}")
    println(s"${m.subMap(30, 30)} ${m.subMap(30, true, 30, true)} ${m.subMap(30, false, 30, true)}")
    attempt("subMap from above to")(m.subMap(50, 20))
    attempt("subMap from above to, both inclusive")(m.subMap(50, true, 40, true))

    // The navigation of a bounded view at, inside and outside its bounds.
    val v = m.subMap(20, true, 60, false)
    for k <- List(5, 20, 25, 50, 55, 60, 65, 95) do
      println(s"$k: ${v.lowerKey(k)} ${v.floorKey(k)} ${v.ceilingKey(k)} ${v.higherKey(k)} ${v.lowerEntry(k)} ${v.higherEntry(k)}")
    println(s"${v.firstKey()} ${v.lastKey()} ${v.firstEntry()} ${v.lastEntry()} ${v.size()} ${v.isEmpty()} ${v.comparator()}")

    // Writes through; out of range: refused by `put`, absent to a lookup or a removal.
    v.put(30, "thirty")
    v.put(55, "fifty-five")
    attempt("put below")(v.put(10, "x"))
    attempt("put at the exclusive bound")(v.put(60, "x"))
    attempt("put above")(v.put(99, "x"))
    println(s"${v.get(10)} ${v.containsKey(10)} ${v.remove(10)} ${v.get(70)} ${v.containsKey(55)} ${m.get(10)} ${m.get(55)}")
    println(s"$v ${v.size()}")
    println(m)
    m.put(45, "forty-five")
    m.remove(20)
    println(s"$v ${v.size()} ${v.firstKey()}")

    // Views of a view: bounds inside its range, or at its exclusive bound when exclusive too.
    println(s"${v.headMap(50, false).tailMap(30, false)} ${v.headMap(60)} ${v.tailMap(20)} ${v.subMap(40, 55)}")
    attempt("headMap past the bound")(v.headMap(70))
    attempt("headMap at the exclusive bound, inclusive")(v.headMap(60, true))
    attempt("tailMap below the bound")(v.tailMap(10))
    attempt("subMap from out of range")(v.subMap(10, 40))
    attempt("subMap to out of range")(v.subMap(30, true, 60, true))
    val narrow = m.subMap(30, false, 50, false)
    attempt("tailMap at an exclusive bound, inclusive")(narrow.tailMap(30, true))
    println(s"${narrow.tailMap(30, false)} ${narrow.headMap(50, false)}")

    // The descending map and key set are views.
    val d = m.descendingMap()
    println(s"$d ${d.firstKey()} ${d.lastKey()} ${d.firstEntry()} ${d.lastEntry()}")
    println(s"${d.headMap(50)} ${d.tailMap(50)} ${d.headMap(50, true)} ${d.subMap(70, 30)} ${d.subMap(70, false, 30, true)}")
    for k <- List(5, 10, 45, 50, 90, 95) do
      println(s"desc $k: ${d.lowerKey(k)} ${d.floorKey(k)} ${d.ceilingKey(k)} ${d.higherKey(k)}")
    attempt("descending subMap from below to")(d.subMap(30, 70))
    d.put(35, "thirty-five")
    println(s"${m.get(35)} ${d.remove(35)} ${m.containsKey(35)}")
    val dv = v.descendingMap()
    println(s"$dv ${dv.firstKey()} ${dv.descendingMap()} ${dv.headMap(40)} ${dv.tailMap(40, false)}")
    attempt("descending view put out of range")(dv.put(65, "x"))
    println(s"${m.descendingKeySet()} ${v.descendingKeySet()} ${m.navigableKeySet().descendingSet()}")
    val dk = m.descendingKeySet()
    println(s"${dk.first()} ${dk.pollFirst()} ${dk.pollLast()} ${dk.headSet(60)} ${dk.lower(60)} ${dk.higher(60)}")
    println(m)
    val dsi = m.descendingKeySet().iterator()
    dsi.next()
    dsi.remove()
    println(s"$m ${m.lastKey()}")
    val ddi = m.navigableKeySet().descendingIterator()
    val seen = new StringBuilder
    while ddi.hasNext do seen.append(ddi.next()).append(" ")
    println(seen.toString.trim)

    // Key sets remove and add nothing.
    val k = tens()
    val ks = k.subMap(20, 70).keySet()
    println(s"${ks.remove(40)} ${ks.remove(80)} ${ks.remove(40)} ${ks.contains(30)} ${ks.contains(80)} $ks $k")
    attempt("add to a view's key set")(ks.add(45))
    attempt("add to the map's key set")(k.keySet().add(5))
    attempt("add to the map's navigable key set")(k.navigableKeySet().add(5))
    attempt("add to the descending key set")(k.descendingKeySet().add(5))
    attempt("add to the values")(k.values().add("x"))
    attempt("add to the entries")(k.entrySet().add(new ju.AbstractMap.SimpleImmutableEntry(5, "x")))
    val ki = k.navigableKeySet().iterator()
    attempt("remove before next")(ki.remove())
    while ki.hasNext do if ki.next() % 20 == 0 then ki.remove()
    attempt("remove twice")(ki.remove())
    println(k)
    k.headMap(50).keySet().clear()
    println(k)
    k.navigableKeySet().tailSet(70, true).clear()
    println(s"$k ${k.size()}")
    val ksub = tens().navigableKeySet().subSet(20, true, 60, false)
    println(s"$ksub ${ksub.first()} ${ksub.last()} ${ksub.pollFirst()} ${ksub.pollLast()} $ksub ${ksub.ceiling(5)} ${ksub.floor(95)}")

    // Entries of a view write through; removing a mapping by its entry.
    val e = tens()
    val es = e.tailMap(60).entrySet()
    val ei = es.iterator()
    while ei.hasNext do
      val en = ei.next()
      en.setValue(en.getValue.toUpperCase)
    println(e)
    println(s"${es.remove(new ju.AbstractMap.SimpleImmutableEntry(70, "V7"))} ${es.remove(new ju.AbstractMap.SimpleImmutableEntry(80, "v8"))} ${es.remove(new ju.AbstractMap.SimpleImmutableEntry(10, "v1"))} ${es.contains(new ju.AbstractMap.SimpleImmutableEntry(90, "V9"))} ${es.contains(new ju.AbstractMap.SimpleImmutableEntry(10, "v1"))}")
    println(s"$e ${es.size()}")
    val pv = e.headMap(40, false)
    println(s"${pv.pollFirstEntry()} ${pv.pollLastEntry()} ${pv.pollLastEntry()} ${pv.pollFirstEntry()} $e")

    // Bulk removals over duplicate values, through the map and through bounded and descending views.
    def dup(): ju.TreeMap[String, Int] =
      val t = new ju.TreeMap[String, Int]()
      for (key, value) <- List("a" -> 1, "b" -> 2, "c" -> 1, "d" -> 3, "e" -> 1, "f" -> 2, "g" -> 3) do t.put(key, value)
      t
    val ones = ju.Arrays.asList(1)
    val t1 = dup()
    println(s"${t1.values().removeAll(ones)} $t1")
    val t2 = dup()
    println(s"${t2.values().remove(2)} ${t2.values().remove(9)} $t2")
    val t3 = dup()
    println(s"${t3.values().retainAll(ones)} $t3")
    val t4 = dup()
    println(s"${t4.headMap("f").values().removeAll(ones)} $t4")
    val t5 = dup()
    println(s"${t5.descendingMap().values().remove(1)} $t5")
    val t6 = dup()
    println(s"${t6.tailMap("c", true).descendingMap().values().retainAll(ju.Arrays.asList(2, 3))} $t6")
    val t7 = dup()
    println(s"${t7.keySet().retainAll(ju.Arrays.asList("a", "c", "z"))} $t7")
    val t8 = dup()
    println(s"${t8.subMap("b", "f").keySet().retainAll(ju.Arrays.asList("a", "c", "z"))} $t8")
    val t9 = dup()
    println(s"${t9.descendingKeySet().headSet("c").retainAll(ju.Arrays.asList("e", "g"))} $t9")
    val t10 = dup()
    println(s"${t10.keySet().removeAll(ju.Arrays.asList("b", "d", "z"))} $t10")
    val t11 = dup()
    val keep = ju.Arrays.asList[ju.Map.Entry[String, Int]](new ju.AbstractMap.SimpleImmutableEntry("a", 1), new ju.AbstractMap.SimpleImmutableEntry("d", 3), new ju.AbstractMap.SimpleImmutableEntry("f", 9))
    println(s"${t11.entrySet().retainAll(keep)} $t11")
    val t12 = dup()
    println(s"${t12.headMap("e", true).descendingMap().entrySet().retainAll(keep)} $t12")
    val t13 = dup()
    println(s"${t13.entrySet().removeAll(keep)} $t13")
    val t14 = dup()
    println(s"${t14.tailMap("b").values().retainAll(ju.Arrays.asList())} ${t14.values().retainAll(ju.Arrays.asList(1, 2, 3))} $t14")
    val t15 = dup()
    t15.subMap("b", true, "e", true).values().clear()
    t15.descendingMap().headMap("f").entrySet().clear()
    println(s"$t15 ${t15.containsValue(1)} ${t15.containsValue(2)} ${t15.subMap("a", "z").containsValue(1)}")
