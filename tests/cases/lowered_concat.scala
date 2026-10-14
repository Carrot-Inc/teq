//> using platform js
// A chain of `+` inside a lowered body (src/interp/code.rs), which the walker evaluates there: on the JVM
// every operand is evaluated before any is rendered, so an object renders what a later operand changed;
// Scala.js renders each operand as its `+` runs (tests/jvm-expected has the JVM's answer, which the
// interpreter gives).
object Main:
  var x = 0
  val o = new Object { override def toString = { println("render"); x.toString } }
  def bump(n: Int): String = { x = n; "!" }
  def chain(n: Int): String = "s=" + o + bump(n) + n
  def main(args: Array[String]): Unit =
    for i <- 1 to 3 do println(chain(i * 3))
