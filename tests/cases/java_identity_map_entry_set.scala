// An identity map's entry set knows its own entries and removes them, by a mapping of the same
// key and value, by reference, as the JDK's does.
import java.util as ju

final class Key(val name: String)

@main def run =
  val a = new Key("a")
  val m = new ju.IdentityHashMap[Key, String]()
  val one = "one"
  m.put(a, one)
  val entries = m.entrySet()
  val e = entries.iterator().next()
  println(s"${entries.contains(e)} ${e == entries.iterator().next()}")
  println(entries.contains(new java.util.AbstractMap.SimpleImmutableEntry(new Key("a"), one)))
  println(s"${entries.remove(e)} ${m.size()} ${entries.contains(e)}")
