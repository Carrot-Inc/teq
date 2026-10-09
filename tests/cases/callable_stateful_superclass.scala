// A function class whose superclass holds state: the superclass is constructed with its
// arguments, as zio's `Callback extends AtomicBoolean(false) with (ZIO => Unit)` is.
import java.util.concurrent.atomic.AtomicBoolean

class Counter(start: Int):
  var count: Int = start
  def bump(): Int =
    count += 1
    count

final class Once extends AtomicBoolean(false), (Int => Boolean):
  def apply(i: Int): Boolean = compareAndSet(false, true)

final class Tally(from: Int) extends Counter(from), (String => Int):
  def apply(s: String): Int = bump() + s.length

object Main:
  def main(args: Array[String]): Unit =
    val once = new Once
    println(List(once(1), once(2), once(3)))
    println(once.get())
    val t = new Tally(10)
    println(t("ab"))
    println(List("x", "yy").map(t))
    println(t.count)
