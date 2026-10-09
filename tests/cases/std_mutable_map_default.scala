// A mutable map's `withDefaultValue` and `withDefault` answer a missing key and share the entries.
import scala.collection.mutable

@main def main(): Unit =
  val m = mutable.Map.empty[String, Int].withDefaultValue(0)
  m("a") = m("a") + 1
  m("a") = m("a") + 1
  println(m("a"))
  println(m("b"))
  println(m.get("b"))
  println(m.contains("b"))
  val base = mutable.HashMap("x" -> 1)
  val d = base.withDefault(k => k.length)
  d("yy") = 5
  println(d("zzz"))
  println(base("yy"))
  println(d.size)
