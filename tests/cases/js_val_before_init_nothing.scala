// A val of type Nothing read before its initialiser runs: the field holds no value, so the read throws
// NullPointerException; the initialiser then throws its own exception.
class A {
  try { println(later) } catch { case _: NullPointerException => println("early-npe") }
  val later: Nothing = throw new IllegalArgumentException("late")
}
object Main { def main(args: Array[String]): Unit = {
  try { new A } catch { case _: IllegalArgumentException => println("late-exception") }
} }
