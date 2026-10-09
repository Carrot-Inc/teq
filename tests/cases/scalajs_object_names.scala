//> using platform js
// `js.Object.getOwnPropertyNames`, the own keys enumerable or not, and `js.Any.wrapDictionary` as a
// mutable map over a dictionary's entries.
import scala.scalajs.js

@main def main(): Unit =
  val o = js.Dynamic.literal(a = 1, b = "two")
  println(js.Object.getOwnPropertyNames(o).mkString(","))
  println(js.Object.getOwnPropertyNames(js.Array(1, 2).asInstanceOf[js.Object]).mkString(","))
  val f: js.Function0[Int] = () => 1
  println(js.Object.getOwnPropertyNames(f.asInstanceOf[js.Object]).contains("length"))
  val d = js.Dictionary("x" -> 1, "y" -> 2)
  val m = js.Any.wrapDictionary(d)
  println(m.get("x"))
  println(m.get("z"))
  println(m.toList.sortBy(_._1))
