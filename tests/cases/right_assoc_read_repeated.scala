// A right-associative member taking repeated arguments: infix, a tuple of operands is lifted before the receiver;
// written out with a spread, the receiver runs before the spread.
def n(s: String): Int = { println(s); 1 }
class M:
  println("M.init")
  def *:(xs: Int*): Int = { println("M.body"); xs.sum }
def seq(): Seq[Int] = { println("spread"); Seq(1, 2) }
def m(): M = { println("M.receiver"); new M }
@main def run(): Unit =
  println((n("s1"), n("s2")) *: m())
  println(m().*:(seq()*))
