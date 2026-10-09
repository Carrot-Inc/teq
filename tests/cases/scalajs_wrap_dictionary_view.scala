//> using platform js
// `js.Any.wrapDictionary` is a view: the map's updates write through to the dictionary and the
// dictionary's are seen by the map; `js.Array.push` takes several elements and answers the new length.
import scala.scalajs.js

@main def main(): Unit =
  val d = js.Dictionary("x" -> 1, "y" -> 2)
  val m = js.Any.wrapDictionary(d)
  m("z") = 3
  m += ("w" -> 4)
  m.remove("x")
  println(d("z"))
  println(js.Object.keys(d.asInstanceOf[js.Object]).mkString(","))
  d("v") = 5
  println(m.get("v"))
  println(m.size)
  println(m.contains("x"))
  println(m.toList.sortBy(_._1))
  val a = js.Array(1, 2)
  val n: Int = a.push(3, 4)
  val k: Int = a.push(5)
  println(n + " " + k + " " + a.mkString(","))
