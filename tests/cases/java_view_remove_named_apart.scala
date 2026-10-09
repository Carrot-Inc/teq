// Removals through the views of a `java.util.IdentityHashMap` in a program that names none of
// the views' classes. Their `remove` overrides `Collection.remove`, which the output names apart
// from `List.remove(Int)`; the interpreter finds it in a std class entered after the program was
// checked, by the name the output gives it.
import java.util as ju

@main def run =
  val m = new ju.IdentityHashMap[String, String]()
  val k = "key"
  m.put(k, "x")
  println(m.values().remove("x"))
  println(m.entrySet().remove("not an entry"))
  m.put(k, "y")
  println(m.entrySet().remove(new ju.AbstractMap.SimpleImmutableEntry(k, "y")))
  m.put(k, "z")
  println(m.keySet().remove(k))
  println(m.size())
