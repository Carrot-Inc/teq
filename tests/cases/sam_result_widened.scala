// A method eta-expanded into a trait with a single abstract method whose result is wider than
// the method's: the wrapper widens the closure's result to the trait method's (`apply(x: Int):
// Long` over `n(x: Int): Int`), so the Long takes part in Long arithmetic on every target
// (JavaScript's BigInt refuses a number; the JVM unboxes the closure's Integer as one).
trait F:
  def apply(x: Int): Long
trait D:
  def apply(x: Int): Double
object M:
  def n(x: Int): Int = x + 1
  def b(x: Int): Byte = (x % 100).toByte
  val g: F = n
  val h: D = n
  val k: F = b
@main def run(): Unit =
  println(M.g(3) + 1L)
  val l: Long = M.g(3)
  println(l * 2)
  println(M.h(3) + 0.5)
  println(M.k(345) + 1L)
  println(M.g(3) == 4L)
