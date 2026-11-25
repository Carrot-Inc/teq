// A right-associative extension written out with its right operand's default left out, a later clause's default
// left out and an explicit using argument: the right default runs first, then the left operand.
def n(s: String): Int = { println(s); 1 }
def s(s: String): String = { println(s); s }
class X:
  extension (l: Int)
    def +:(r: String = s("D-right"))(z: Int = n("D-later"))(using u: String): Int = { println("body"); l + z }
def x(): X = { println("prefix"); new X }
given u: String = s("using")
@main def run(): Unit = println(x().+:()(n("left"))()(using s("explicit-using")))
