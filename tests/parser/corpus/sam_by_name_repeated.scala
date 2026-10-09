// A lambda implements a single abstract method with a by-name or a repeated parameter: the
// by-name one is evaluated where the body reads it, the repeated one is a `Seq`.
trait Lazy:
  def f(x: => Int): Int
trait Many:
  def f(x: Int*): Int
trait Label:
  def f(prefix: String, xs: Int*): String

@main def main(): Unit =
  var reads = 0
  def next(): Int = { reads += 1; reads }
  val l: Lazy = x => x + x
  println(s"${l.f(next())} $reads")
  val l2: Lazy = x => 0
  println(s"${l2.f(next())} $reads")
  val m: Many = (xs: Seq[Int]) => xs.sum
  val m2: Many = xs => xs.size
  println(s"${m.f(1, 2, 3)} ${m2.f()} ${m2.f(List(4, 5)*)}")
  val t: Label = (p, xs) => p + xs.mkString(",")
  println(t.f("n=", 7, 8))
