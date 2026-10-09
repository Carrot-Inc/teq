// The JDK's shared bodies of its collections: `removeAll` and `retainAll` test each own element
// against the argument (duplicates go too), the defaults of `Map` (`forEach`, `putIfAbsent`,
// `merge`, `compute` and kin, a null result removing the mapping), maps and sets equal by content
// whatever their class with the hashes of `AbstractMap` and `AbstractSet`, an `IdentityHashMap`
// rendered as any map, and a tree copied from a sorted one without comparing a key.
import java.util.{ArrayList, HashMap, HashSet, IdentityHashMap, TreeMap, TreeSet}

class Counting extends java.util.Comparator[String]:
  var calls = 0
  def compare(a: String, b: String): Int =
    calls += 1
    a.compareTo(b)

object Main:
  def main(args: Array[String]): Unit =
    val a = new ArrayList[String]()
    a.add("a"); a.add("b"); a.add("a"); a.add("c")
    println(a.removeAll(java.util.List.of("a")))
    println(a)
    println(a.retainAll(java.util.List.of("c", "z")))
    println(a)
    println(a.removeIf(_ == "q"))
    val hs = new HashSet[Int]()
    (1 to 5).foreach(hs.add(_))
    println(hs.removeAll(java.util.List.of(2, 4, 9)))
    println(hs.retainAll(java.util.List.of(1, 3)))
    println(hs)

    val m: java.util.Map[String, Int] = new HashMap[String, Int]()
    println(m.putIfAbsent("x", 1))
    println(m.putIfAbsent("x", 5))
    println(m.merge("x", 2, (p, q) => p + q))
    println(m.merge("y", 7, (p, q) => p + q))
    println(m.compute("x", (_, v) => v * 10))
    println(m.computeIfPresent("y", (_, v) => v + 1))
    println(m.computeIfPresent("z", (_, v) => v + 1))
    println(m.getOrDefault("z", -1))
    println(m.replace("y", 3))
    println(m.replace("x", 30, 31))
    println(m.remove("y", 4))
    println(m.remove("y", 3))
    m.remove("x")
    m.put("w", 1)
    m.put("x", 2)
    m.forEach((k, v) => println(k + " -> " + v))
    val sm = new HashMap[String, String]()
    sm.put("a", "1")
    println(sm.compute("a", (_, _) => null))
    println(sm.containsKey("a"))
    println(sm.merge("b", "2", (_, _) => null))
    println(sm.merge("b", "3", (_, _) => null))
    println(sm.containsKey("b"))

    val hm = new HashMap[String, Int](); hm.put("k", 1); hm.put("j", 2)
    val tm = new TreeMap[String, Int](); tm.put("j", 2); tm.put("k", 1)
    println(hm == tm)
    println(tm == hm)
    println(hm.hashCode == tm.hashCode)
    tm.put("k", 9)
    println(hm == tm)
    val ts = new TreeSet[String](); ts.add("j"); ts.add("k")
    println(ts == hm.keySet())
    println(hm.keySet() == ts)
    println(ts.hashCode == hm.keySet().hashCode)
    println(ts == java.util.List.of("j", "k"))
    val ih = new IdentityHashMap[String, String](); ih.put("p", "q")
    println(ih)

    val c = new Counting
    val src = new TreeMap[String, String](c)
    Seq("d", "b", "a", "c", "e").foreach(k => src.put(k, k.toUpperCase))
    c.calls = 0
    val copy = new TreeMap[String, String](src: java.util.SortedMap[String, String])
    println(c.calls)
    println(copy)
    println(copy.firstKey() + copy.lastKey())
    copy.put("bb", "BB")
    println(copy.headMap("c"))
    val set = new TreeSet[String](c)
    Seq("q", "p", "r").foreach(set.add)
    c.calls = 0
    val setCopy = new TreeSet[String](set: java.util.SortedSet[String])
    println(c.calls)
    println(setCopy)
    val natural = new TreeMap[String, Int](); natural.put("b", 2); natural.put("a", 1)
    val viaPutAll = new TreeMap[String, Int](); viaPutAll.putAll(natural)
    println(viaPutAll)
