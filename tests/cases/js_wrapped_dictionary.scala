//> using platform js
// A dictionary's `map` of pairs and its `addAll` where a dictionary is expected (indigo's asset mapping and scene
// registers): Scala.js's `js.WrappedDictionary`, its companion's `toJSDictionary` converting it back.
import scala.scalajs.js

final class Holder(val d: js.Dictionary[Int])

@main def run(): Unit =
  val legend = js.Dictionary("a" -> 1, "b" -> 2)
  val doubled = new Holder(legend.map { case (k, v) => k -> v * 2 })
  println(doubled.d("a") + doubled.d("b"))
  val added: js.Dictionary[String] = js.Dictionary.empty[String].addAll(List("x" -> "1", "y" -> "2"))
  println(added("y"))
  val keys = legend.map { case (k, v) => k + v }
  println(keys.toList.sorted)
  println(js.Object.keys(added.asInstanceOf[js.Object]).toList)
