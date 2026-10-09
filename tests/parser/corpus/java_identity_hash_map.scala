// `java.util.IdentityHashMap`, keyed by reference, with views the map backs (a clear of its
// values empties it, its key set adds nothing, an iterator removes); a set over it from
// `Collections.newSetFromMap`, as a library keeps the objects it has visited (munit's
// `StackTraces`); `Collections.addAll`.
import java.util as ju

final class Key(val name: String):
  override def equals(o: Any): Boolean = o.isInstanceOf[Key] && o.asInstanceOf[Key].name == name
  override def hashCode: Int = name.hashCode

@main def run =
  val a = new Key("k")
  val b = new Key("k")
  val m = new ju.IdentityHashMap[Key, Int]()
  m.put(a, 1)
  m.put(b, 2)
  println(s"${m.size()} ${m.get(a)} ${m.get(b)} ${m.containsKey(new Key("k"))}")
  println(m.put(a, 3))
  println(m.remove(b))
  println(s"${m.size()} ${m.get(a)} ${m.isEmpty()}")
  val seen = ju.Collections.newSetFromMap(new ju.IdentityHashMap[Key, java.lang.Boolean]())
  println(s"${seen.add(a)} ${seen.add(b)} ${seen.add(a)} ${seen.size()} ${seen.contains(b)}")
  val list = new ju.ArrayList[String]()
  println(ju.Collections.addAll(list, "x", "y"))
  println(list)
  val m2 = new ju.IdentityHashMap[Key, Int]()
  m2.put(a, 1)
  val values = m2.values()
  m2.put(b, 2)
  println(values.size())
  values.clear()
  println(m2.size())
  m2.put(a, 5)
  val keys = m2.keySet()
  try
    keys.add(b)
    println("added")
  catch case _: UnsupportedOperationException => println("unsupported")
  val entries = m2.entrySet()
  m2.put(b, 6)
  println(entries.size())
  val it = keys.iterator()
  val first = it.next()
  it.remove()
  println(s"${m2.size()} ${m2.containsKey(first)} ${keys.contains(first)}")
