//> using platform js
// `js.UndefOr`'s `collect` as Scala.js implements it: an empty value collects nothing, a defined one goes through one
// `applyOrElse`, so a guard with an effect runs once, and a value the cases refuse gives `undefined`.
import scala.scalajs.js

@main def main(): Unit =
  var n = 0
  val x: js.UndefOr[Int] = 1
  val y = x.collect { case v if { n += 1; true } => v + 1 }
  println(y.getOrElse(0))
  println(n)
  val refused = x.collect { case v if { n += 1; false } => v }
  println(refused.isEmpty)
  println(n)
  val empty: js.UndefOr[Int] = js.undefined
  println(empty.collect { case v if { n += 1; true } => v }.isEmpty)
  println(n)
