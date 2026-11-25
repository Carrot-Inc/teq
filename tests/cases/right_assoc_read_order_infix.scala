// A right-associative extension infix, then a later clause and an explicit using argument: the left operand, the
// right one, then the later clause.
def n(s: String): Int = { println(s); 1 }
def s(s: String): String = { println(s); s }
class X:
  extension (l: Int)
    def +:(r: String = s("D-right"))(z: Int = n("D-later"))(using u: String): Int = { println("body"); l + z }
def x(): X = { println("prefix"); new X }
given u: String = s("using")
@main def run(): Unit =
  val q = x()
  import q.*
  println((n("left") +: s("right"))(n("later"))(using s("explicit-using")))
