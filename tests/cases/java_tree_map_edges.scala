// Edges of `java.util.TreeMap` and `TreeSet` against the JDK's: a null argument to a bulk
// operation of a view fails before anything is removed, an empty view's too; `removeIf` removes
// the occurrence it tested among repeated values and takes a predicate over a supertype; a map, a
// view and a values collection that contain themselves print as such; the views are made once; an
// iterator's `remove` after the map lost the key under it removes nothing more (the JDK fails fast
// there, which the program catches without printing); values and entries compare as the JDK's
// `equals` (a NaN is itself, `-0.0` is not `0.0`); `toArray` fills a destination large enough;
// and a reversed natural ordering fails for a null key with `NullPointerException`.
import java.util as ju

object Main:
  def attempt(label: String)(body: => Any): Unit =
    val out =
      try "" + body
      catch case e: RuntimeException => e.getClass.getName
    println(s"$label: $out")

  def predicate[T](f: T => Boolean): ju.function.Predicate[T] = new ju.function.Predicate[T]:
    def test(t: T): Boolean = f(t)

  def main(args: Array[String]): Unit =
    val m = new ju.TreeMap[String, String]()
    m.put("a", "one")
    attempt("values removeAll null")(m.values().removeAll(null))
    attempt("values retainAll null")(m.values().retainAll(null))
    attempt("keys removeAll null")(m.keySet().removeAll(null))
    attempt("keys retainAll null")(m.keySet().retainAll(null))
    attempt("entries removeAll null")(m.entrySet().removeAll(null))
    attempt("entries retainAll null")(m.entrySet().retainAll(null))
    attempt("an empty view's keys removeAll null")(m.tailMap("z", true).keySet().removeAll(null))
    attempt("an empty view's values retainAll null")(m.tailMap("z", true).values().retainAll(null))
    attempt("values removeIf null")(m.values().removeIf(null))
    val s = new ju.TreeSet[String]()
    s.add("x")
    attempt("set removeAll null")(s.removeAll(null))
    attempt("set retainAll null")(s.retainAll(null))
    println(s"$m $s")

    // `removeIf` removes as it goes, the tested occurrence of a repeated value.
    val d = new ju.TreeMap[String, String]()
    for (k, v) <- List("a" -> "same", "b" -> "same", "c" -> "other", "d" -> "same", "e" -> "same") do d.put(k, v)
    var calls = 0
    println(d.values().removeIf(predicate[String] { _ =>
      calls += 1
      calls == 2 || calls == 4
    }))
    println(s"$d $calls")
    println(d.descendingMap().headMap("b", true).keySet().removeIf(predicate[String](_ != "c")))
    println(d)
    d.put("f", "other")
    d.put("g", "same")
    println(d.entrySet().removeIf(predicate[ju.Map.Entry[String, String]](_.getValue == "other")))
    println(d)
    val t = new ju.TreeSet[Int]()
    for i <- 1 to 8 do t.add(i)
    println(s"${t.removeIf(predicate[Int](_ % 3 == 0))} $t ${t.headSet(5).removeIf(predicate[Int](_ % 2 == 0))} $t")

    // A map, a view and a values collection that contain themselves.
    val self = new ju.TreeMap[String, Any]()
    self.put("me", self)
    self.put("n", 1)
    println(self)
    println(self.descendingMap())
    println(self.headMap("n"))
    println(self.entrySet())
    val w = new ju.TreeMap[String, Any]()
    w.put("a", 1)
    w.put("b", w.values())
    println(w.values())
    println(w)
    println(s"${w.values() eq w.values()} ${w.keySet() eq w.navigableKeySet()} ${w.entrySet() eq w.entrySet()} ${w.descendingMap() eq w.descendingMap()}")

    // An iterator's removal after the map lost the key it gave last.
    val lost = new ju.TreeMap[String, String]()
    lost.put("a", "one")
    lost.put("b", "two")
    val it = lost.keySet().iterator()
    println(it.next())
    lost.remove("a")
    println(it.hasNext)
    try it.remove()
    catch case _: ju.ConcurrentModificationException => ()
    println(s"${lost.size()} ${lost.keySet()} ${lost.get("b")} ${lost.firstKey()}")
    lost.put("c", "three")
    println(s"${lost.size()} $lost")

    // Values and entries compare as the JDK's `equals`: a NaN is itself, `-0.0` is not `0.0`.
    val nan = new ju.AbstractMap.SimpleImmutableEntry("k", Double.NaN)
    println(s"${nan.equals(nan)} ${nan.equals(new ju.AbstractMap.SimpleImmutableEntry("k", Double.NaN))}")
    val zeros = new ju.TreeMap[String, Double]()
    zeros.put("k", -0.0)
    zeros.put("n", Double.NaN)
    println(s"${zeros.containsValue(0.0)} ${zeros.containsValue(-0.0)} ${zeros.containsValue(Double.NaN)}")
    println(s"${zeros.entrySet().remove(new ju.AbstractMap.SimpleImmutableEntry("k", 0.0))} ${zeros.entrySet().contains(new ju.AbstractMap.SimpleImmutableEntry("n", Double.NaN))} ${zeros.values().remove(0.0)} ${zeros.size()}")
    println(s"${zeros.firstEntry() == new ju.AbstractMap.SimpleImmutableEntry("k", -0.0)} ${zeros.firstEntry().hashCode == new ju.AbstractMap.SimpleImmutableEntry("k", -0.0).hashCode}")

    // `removeIf` takes a predicate over a supertype of the elements.
    val anything: ju.function.Predicate[Any] = predicate[Any](_.toString.startsWith("b"))
    val bs = new ju.TreeSet[String]()
    for w <- List("apple", "banana", "blueberry", "cherry") do bs.add(w)
    println(s"${bs.removeIf(anything)} $bs")
    val bm = new ju.TreeMap[String, Int]()
    for (w, i) <- List("bean" -> 1, "corn" -> 2, "beet" -> 3) do bm.put(w, i)
    println(s"${bm.keySet().removeIf(anything)} $bm")

    // `toArray` fills a destination large enough and ends the elements with a null.
    val arr = new ju.TreeMap[String, String]()
    arr.put("a", "one")
    arr.put("b", "two")
    val dest = Array("x", "x", "x", "x")
    val filled = arr.values().toArray(dest)
    println(s"${filled eq dest} ${dest.mkString(",")}")
    val exact = Array("x", "x")
    println(s"${arr.keySet().toArray(exact) eq exact} ${exact.mkString(",")}")
    val small = Array("x")
    val grown = arr.navigableKeySet().descendingSet().toArray(small)
    println(s"${grown eq small} ${grown.mkString(",")} ${small.mkString(",")}")
    attempt("toArray null")(arr.values().toArray(null: Array[String]).length)

    // A reversed natural ordering fails for a null key as the JDK's does.
    val source = new ju.TreeMap[String, String]()
    val reverse = source.descendingMap().comparator()
    attempt("reverse compare null first")(reverse.compare(null, "a"))
    attempt("reverse compare null second")(reverse.compare("a", null))
    val copied = new ju.TreeMap[String, String](source.descendingMap())
    copied.put("a", "one")
    attempt("put null into a reversed copy")(copied.put(null, "two"))
    attempt("floorKey null of a reversed copy")(copied.floorKey(null))
    println(copied)
