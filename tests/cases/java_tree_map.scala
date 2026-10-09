// `java.util.TreeMap` against the JDK's: natural ordering (strings, longs, doubles with their
// signed zeros and NaN) and a comparator's, every navigation at an absent key, at the first and
// last keys and on an empty map, the polls of a map of one and of none, the entries (the copies
// the navigation hands out refuse `setValue` and keep their value, the entry set's write through),
// `putAll` from a `HashMap`, the constructors (built from a sorted map, the comparator kept; from
// a map, natural ordering), and the keys natural ordering refuses: null and a value that is not
// `Comparable`, eagerly for a lookup, only once there is a key to compare with for a navigation,
// and a key of another kind than the map's (a number among strings).
import java.util as ju

final case class Point(x: Int, y: Int)

object Main:
  def show(e: ju.Map.Entry[?, ?]): String = if e == null then "none" else s"${e.getKey}=${e.getValue}"

  def attempt(label: String)(body: => Any): Unit =
    val out =
      try "" + body
      catch
        case e: IllegalArgumentException => s"IllegalArgumentException(${e.getMessage})"
        case e: RuntimeException => e.getClass.getName
    println(s"$label: $out")

  def navigation(m: ju.NavigableMap[String, Int], key: String): Unit =
    println(s"$key: lower ${m.lowerKey(key)} ${show(m.lowerEntry(key))}, floor ${m.floorKey(key)} ${show(m.floorEntry(key))}, " +
      s"ceiling ${m.ceilingKey(key)} ${show(m.ceilingEntry(key))}, higher ${m.higherKey(key)} ${show(m.higherEntry(key))}")

  def main(args: Array[String]): Unit =
    val m = new ju.TreeMap[String, Int]()
    for (k, v) <- List("m" -> 13, "c" -> 3, "x" -> 24, "a" -> 1, "q" -> 17) do m.put(k, v)
    println(m)
    println(s"${m.size()} ${m.firstKey()} ${m.lastKey()} ${show(m.firstEntry())} ${show(m.lastEntry())} ${m.comparator()}")
    for k <- List("0", "a", "b", "c", "m", "n", "x", "z") do navigation(m, k)
    println(s"${m.get("q")} ${m.get("r")} ${m.containsKey("x")} ${m.containsKey("y")} ${m.containsValue(24)} ${m.containsValue(25)}")
    println(s"${m.put("q", 170)} ${m.put("r", 18)} ${m.remove("c")} ${m.remove("c")} ${m.getOrDefault("c", -1)}")
    println(s"$m ${m.keySet()} ${m.values()} ${m.entrySet()}")
    println(s"${show(m.pollFirstEntry())} ${show(m.pollLastEntry())} $m")

    // An empty map, and a map of one polled to none from either end.
    val empty = new ju.TreeMap[String, Int]()
    navigation(empty, "a")
    println(s"${empty.firstEntry()} ${empty.lastEntry()} ${empty.pollFirstEntry()} ${empty.pollLastEntry()} ${empty.isEmpty()} $empty")
    attempt("firstKey of none")(empty.firstKey())
    attempt("lastKey of none")(empty.lastKey())
    val one = new ju.TreeMap[String, Int]()
    one.put("only", 1)
    navigation(one, "only")
    println(s"${show(one.pollFirstEntry())} ${one.size()} ${one.pollFirstEntry()}")
    one.put("only", 2)
    println(s"${show(one.pollLastEntry())} ${one.isEmpty()} ${one.pollLastEntry()}")

    // The navigation's entries are copies; the entry set's are the mappings.
    val copy = m.firstEntry()
    attempt("setValue of a copy")(copy.setValue(0))
    m.put(copy.getKey, 100)
    println(s"${show(copy)} ${m.get(copy.getKey)}")
    val live = m.entrySet().iterator().next()
    println(s"${live.setValue(200)} ${show(live)} $m")
    println(s"${live == m.firstEntry()} ${live.hashCode == m.firstEntry().hashCode} ${m.firstEntry() == new ju.AbstractMap.SimpleImmutableEntry("a", 1)}")
    println(s"${m.entrySet().contains(new ju.AbstractMap.SimpleImmutableEntry("a", 200))} ${m.entrySet().contains(new ju.AbstractMap.SimpleImmutableEntry("a", 1))}")
    val polled = m.pollFirstEntry()
    attempt("setValue of a polled entry")(polled.setValue(1))
    println(s"${show(polled)} $m")

    // A comparator's order, kept by a map built from the sorted map, dropped by one built from a map.
    val byLength: ju.Comparator[String] = (a, b) => if a.length != b.length then a.length - b.length else a.compareTo(b)
    val c = new ju.TreeMap[String, Int](byLength)
    for w <- List("pear", "fig", "banana", "kiwi", "apple", "date", "fig") do c.put(w, w.length)
    println(c)
    println(s"${c.ceilingKey("zzzz")} ${c.floorKey("aaaaa")} ${c.lowerKey("fig")} ${c.higherKey("banana")} ${c.headMap("kiwi")} ${c.comparator() eq byLength}")
    val sorted = new ju.TreeMap[String, Int](c)
    sorted.put("plum", 4)
    println(s"$sorted ${sorted.comparator() eq byLength}")
    val asMap: ju.Map[String, Int] = c
    val natural = new ju.TreeMap[String, Int](asMap)
    println(s"$natural ${natural.comparator()}")
    val reverse = new ju.TreeMap[Int, String](Ordering.Int.reverse)
    for i <- List(3, -1, 7, 0) do reverse.put(i, "n" + i)
    println(s"$reverse ${reverse.firstKey()} ${reverse.ceilingKey(5)} ${reverse.floorKey(5)}")

    // `putAll` and the constructor from a hash map.
    val h = new ju.HashMap[String, Int]()
    for w <- List("delta", "alpha", "charlie", "bravo") do h.put(w, w.length)
    val t = new ju.TreeMap[String, Int]()
    t.put("echo", 4)
    t.putAll(h)
    println(s"$t ${new ju.TreeMap[String, Int](h)}")

    // Natural ordering of numbers.
    val longs = new ju.TreeMap[Long, String]()
    for k <- List(3000000000L, -5L, Long.MaxValue, Long.MinValue, 0L, 9007199254740993L, 9007199254740992L) do longs.put(k, "")
    println(longs.keySet())
    val ints = new ju.TreeMap[Int, String]()
    for k <- List(Int.MaxValue, -1, Int.MinValue, 1, 0) do ints.put(k, "i" + k)
    println(s"$ints ${ints.lowerKey(Int.MinValue)} ${ints.higherKey(0)} ${ints.get(5)}")
    val doubles = new ju.TreeMap[Double, Int]()
    for d <- List(1.5, -0.0, 0.0, Double.NaN, Double.NegativeInfinity, -2.5) do doubles.put(d, 1)
    // Whole doubles print as JavaScript prints them on that platform: the zeros by their sign.
    def label(d: Double): String = if d.isNaN then "NaN" else if d == 0 then (if 1 / d < 0 then "-0" else "+0") else d.toString
    val labels = scala.collection.mutable.ListBuffer.empty[String]
    val dk = doubles.keySet().iterator()
    while dk.hasNext do labels += label(dk.next())
    println(s"${labels.mkString(" ")} ${doubles.size()} ${doubles.containsKey(Double.NaN)} ${label(doubles.ceilingKey(-0.0))} ${label(doubles.higherKey(0.0))}")

    // The keys natural ordering refuses.
    val n = new ju.TreeMap[String, Int]()
    attempt("put null into none")(n.put(null, 1))
    attempt("get null of none")(n.get(null))
    attempt("containsKey null of none")(n.containsKey(null))
    attempt("remove null of none")(n.remove(null))
    attempt("ceilingKey null of none")(n.ceilingKey(null))
    attempt("headMap null of none")(n.headMap(null))
    n.put("a", 1)
    attempt("put null")(n.put(null, 1))
    attempt("get null")(n.get(null))
    attempt("floorKey null")(n.floorKey(null))
    attempt("tailMap null")(n.tailMap(null, true))
    println(n)
    val anything = new ju.TreeMap[Any, Int]()
    attempt("put a point into none")(anything.put(Point(1, 2), 1))
    attempt("get a point of none")(anything.get(Point(1, 2)))
    attempt("lowerKey a point of none")(anything.lowerKey(Point(1, 2)))
    anything.put("k", 1)
    attempt("put a point")(anything.put(Point(1, 2), 2))
    attempt("higherKey a point")(anything.higherKey(Point(1, 2)))
    println(anything)
    val mixed = new ju.TreeMap[Any, Int]()
    mixed.put("a", 1)
    attempt("put a number among strings")(mixed.put(1, 2))
    attempt("put a long among strings")(mixed.put(1L, 2))
    attempt("ceilingKey a number among strings")(mixed.ceilingKey(1))
    mixed.clear()
    mixed.put(1, 1)
    attempt("put a string among numbers")(mixed.put("a", 2))
    attempt("put a long among ints")(mixed.put(2L, 2))
    attempt("put a boolean among ints")(mixed.put(true, 2))
    println(mixed)

    // A comparator that orders null decides for itself.
    val nullsFirst: ju.Comparator[String] = (a, b) => if a == null then (if b == null then 0 else -1) else if b == null then 1 else a.compareTo(b)
    val withNull = new ju.TreeMap[String, Int](nullsFirst)
    withNull.put("b", 2)
    withNull.put(null, 0)
    withNull.put("a", 1)
    println(withNull)
    println(s"${withNull.get(null)} ${withNull.firstKey()} ${withNull.higherKey(null)} ${withNull.remove(null)}")
    println(withNull)

    // Many keys in and out, against the expected order.
    val big = new ju.TreeMap[Int, Int]()
    for i <- 0 until 2000 do big.put((i * 7919) % 2000, i)
    for i <- 0 until 2000 by 3 do big.remove((i * 31) % 2000)
    val expected = (0 until 2000).filterNot(k => (0 until 2000 by 3).exists(i => (i * 31) % 2000 == k)).toList
    val got = scala.collection.mutable.ListBuffer.empty[Int]
    val it = big.keySet().iterator()
    while it.hasNext do got += it.next()
    println(s"${got.toList == expected} ${big.size()} ${expected.size} ${big.firstKey()} ${big.lastKey()} ${big.floorKey(1000)} ${big.ceilingKey(1000)}")
