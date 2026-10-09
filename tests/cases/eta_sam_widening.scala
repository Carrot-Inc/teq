// A method eta-expanded into a trait with a single abstract method takes the trait method's
// parameter types, from which a number widens to the method's, and its result widens to the
// trait method's (dotc's `(x: Int) => m(x)`).
trait F:
  def apply(x: Int): Long
trait G:
  def run(x: Int): Unit
object M:
  def m(x: Long): Long = x * 2
  def n(x: Int): Int = x + 1
  def p(x: Double, y: Int): Long = (x * y).toLong
  val f: F = m
  val g: F = n
  val h: G = n
trait H:
  def apply(x: Int, y: Int): Long
@main def run(): Unit =
  println(M.f(3))
  println(M.g(3))
  M.h.run(1)
  val k: H = M.p
  println(k(3, 4))
