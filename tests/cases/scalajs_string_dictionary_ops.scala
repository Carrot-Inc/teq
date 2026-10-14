//> using platform js
// Scala.js's facade members Laminar reaches: `js.RegExp`'s constructor parameters by their names (`flags = "g"`),
// `JSStringOps`' JavaScript methods of a string (`jsSplit`, `jsReplace`, `jsSlice`), a dictionary's `filter` (a map,
// as `js.Any.wrapDictionary` makes it) and `js.UndefOr`'s `collect`.
import scala.scalajs.js
import scala.scalajs.js.JSStringOps.*

@main def main(): Unit =
  val separator = new js.RegExp(" ", flags = "g")
  println("a b  c".jsReplace(separator, "-"))
  println("x,y,,z".jsSplit(",").toList.filter(_.nonEmpty).mkString("|"))
  println("x;y;z".jsSplit(";", 2).length)
  println("abcdef".jsSlice(1, 3) + " " + "abcdef".jsSlice(-2))
  println("ab".jsIndexOf("b") + " " + "abab".jsLastIndexOf("a"))
  val flags = js.Dictionary("on" -> true, "off" -> false, "also" -> true)
  println(flags.filter(_._2).keys.toList.sorted.mkString(","))
  val present: js.UndefOr[Any] = true
  val absent: js.UndefOr[Any] = js.undefined
  println(present.collect { case b: Boolean => !b })
  println(absent.collect { case b: Boolean => b }.isEmpty)
  val text: js.UndefOr[Any] = "t"
  println(text.collect { case b: Boolean => b }.isEmpty)
