// jars: scala-library
// std: scala-library
// A value class's by-name default: its getter on the companion, `f$default$1$extension(J)J`, returns the value, and
// the caller passes a thunk that calls it, the receiver and the earlier clauses' arguments captured, so the default
// is evaluated at each read of the parameter.
var count = 0
def tick(): Long = { count += 1; count.toLong }
class V(val x: Long) extends AnyVal:
  def f(y: => Long = x + 2L): Long = y + y
  def g(a: Long)(b: => Long = a + x + tick()): Long = b * 10 + b
  def h(s: String, n: => Int = 3): String = s * n
object Main:
  def main(args: Array[String]): Unit =
    println(new V(10L).f())
    println(new V(10L).f(5L))
    println(count)
    println(new V(1L).g(100L)())
    println(count)
    println(new V(1L).h("ab"))
