// A block whose statements all emit nothing is written as its result would be written in the block's place: a
// JavaScript object literal as an arrow's body, as a receiver, at a statement's start and in a condition.
// The expectation is scalac 3.8.4's under Scala.js 1.22.0 (`scala-cli run --js --js-version 1.22.0 --scala 3.8.4`),
// not written by hand.
import scala.scalajs.js

trait P extends js.Object { val x: Int }
object Quiet

object Main {
  @annotation.nowarn("msg=pure expression")
  def main(args: Array[String]): Unit = {
    val f = () => { 1; new P { val x = 2 } }
    println(f().x)
    println({ 1; new P { val x = 3 } }.x)
    val g: js.Function0[P] = () => { Quiet; new P { val x = 4 } }
    println(g().x)
    { 5; new P { val x = 5 } }
    val h = () => { (); js.Dynamic.literal(x = 6) }
    println(h().x)
    println(if ({ 7; new P { val x = 7 } }.x > 0) "positive" else "not")
    println("end")
  }
}
