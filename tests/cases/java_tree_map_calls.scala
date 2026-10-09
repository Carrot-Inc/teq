// What the sorted collections call, against the JDK's: `toArray(a)` takes `size()` as a hint
// and reads the iterator to its end (an iterator that gives more grows the result, one that gives
// fewer trims it, copies it back into `a` or ends `a`'s elements with a null); a value search calls
// `equals` on the value searched for (`containsValue`, `values().contains`, a view's
// `values().remove`), while the entry sets and the whole map's `values().remove` call it on the
// map's value; and `equals` is called even on an object compared with itself, where Scala's `==`
// would answer without the call.
import java.util as ju

// A set whose iterator adds or removes elements before it starts.
class Growing(extra: String*) extends ju.TreeSet[String]:
  override def iterator(): ju.Iterator[String] =
    extra.foreach(add)
    super.iterator()
class Shrinking(gone: String*) extends ju.TreeSet[String]:
  override def iterator(): ju.Iterator[String] =
    gone.foreach(remove)
    super.iterator()

class Loud(val id: String):
  override def equals(o: Any): Boolean =
    println("equals " + id)
    o.isInstanceOf[Loud]
  override def hashCode: Int = 1

class Counted:
  var calls = 0
  override def equals(o: Any): Boolean =
    calls += 1
    o.isInstanceOf[Counted]
  override def hashCode: Int = 1

class NeverEqual:
  override def equals(o: Any): Boolean = false
  override def hashCode: Int = 2

object Main:
  def show(label: String, r: Array[String], a: Array[String]): Unit =
    println(s"$label: ${r.mkString(",")} ${r eq a} ${a.mkString(",")}")

  def main(args: Array[String]): Unit =
    val empty = Array.empty[String]
    val g1 = new Growing("b")
    g1.add("a")
    show("grows from none", g1.toArray(empty), empty)
    val g2 = new Growing("b")
    g2.add("a")
    val one = Array("x")
    show("grows past a full array", g2.toArray(one), one)
    val g3 = new Growing("b", "c", "d", "e", "f")
    g3.add("a")
    val small = Array("x")
    show("grows several times", g3.toArray(small), small)
    val s1 = new Shrinking("b")
    s1.add("a")
    s1.add("b")
    val shortArray = Array("x")
    show("shrinks into a short array", s1.toArray(shortArray), shortArray)
    val s2 = new Shrinking("b")
    s2.add("a")
    s2.add("b")
    val longArray = Array("x", "x", "x")
    show("shrinks in a long array", s2.toArray(longArray), longArray)
    val s3 = new Shrinking("b")
    s3.add("a")
    s3.add("b")
    show("shrinks from none", s3.toArray(empty), empty)
    val s4 = new Shrinking("a", "b")
    s4.add("a")
    s4.add("b")
    val nothing = Array.empty[String]
    show("shrinks to nothing", s4.toArray(nothing), nothing)

    // The receiver of `equals`, per operation, as the JDK's.
    val m = new ju.TreeMap[String, Loud]()
    val stored = new Loud("stored")
    val query = new Loud("query")
    m.put("a", stored)
    val entry = new ju.AbstractMap.SimpleImmutableEntry("a", query)
    println("containsValue"); println(m.containsValue(query))
    println("values contains"); println(m.values().contains(query))
    println("view containsValue"); println(m.tailMap("a", true).containsValue(query))
    println("descending values contains"); println(m.descendingMap().values().contains(query))
    println("entrySet contains"); println(m.entrySet().contains(entry))
    println("view entrySet contains"); println(m.tailMap("a", true).entrySet().contains(entry))
    println("entrySet remove"); println(m.entrySet().remove(entry)); m.put("a", stored)
    println("view entrySet remove"); println(m.headMap("b").entrySet().remove(entry)); m.put("a", stored)
    println("values remove"); println(m.values().remove(query)); m.put("a", stored)
    println("view values remove"); println(m.tailMap("a", true).values().remove(query)); m.put("a", stored)
    println("descending values remove"); println(m.descendingMap().values().remove(query)); m.put("a", stored)
    println("entry equals"); println(m.firstEntry().equals(entry))
    println("entry equals itself"); println(entry.equals(entry))

    // `equals` is called on an object compared with itself.
    val c = new Counted
    val e = new ju.AbstractMap.SimpleImmutableEntry[String, Counted]("k", c)
    println(s"${e.equals(e)} ${c.calls}")
    val cm = new ju.TreeMap[String, Counted]()
    cm.put("k", c)
    println(s"${cm.containsValue(c)} ${c.calls} ${cm.values().remove(c)} ${c.calls} ${cm.size()}")
    val never = new NeverEqual
    val nm = new ju.TreeMap[String, NeverEqual]()
    nm.put("k", never)
    val ne = new ju.AbstractMap.SimpleImmutableEntry[String, NeverEqual]("k", never)
    println(s"${nm.containsValue(never)} ${nm.values().remove(never)} ${nm.entrySet().contains(ne)} ${ne.equals(ne)} ${nm.firstEntry() == nm.firstEntry()} ${nm.size()}")
