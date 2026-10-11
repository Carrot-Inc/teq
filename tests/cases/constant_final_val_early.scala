// A constant `final val`'s getter answers the constant whatever the constructor has run
// (dotty's `Memoize`), a trait's body reading it first; its right side is still the
// constructor's work, run once where the val stands (`Constructors`).
trait T:
  def k: Int
  def s: String
  println(s"$k $s")
class C extends T:
  final val k: 7 = { println("init k"); 7 }
  final val s: "seven" = { println("init s"); "seven" }

trait U:
  def n: Int
  val seen: Int = n + 1
object O extends U:
  final val n: 41 = { println("init n"); 41 }

@main def run(): Unit =
  val c = new C
  println(c.k + c.s.length)
  println(O.seen)
  println(O.n)
