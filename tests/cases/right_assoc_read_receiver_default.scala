// A right-associative extension whose receiver has a default, written out with the receiver left out: the right
// argument runs first, then the receiver's default.
def n(s: String): Int = { println(s); 1 }
def s(s: String): String = { println(s); s }
class E:
  extension (l: Int = n("D-left"))
    def +:(r: String): Int = { println("body"); l + r.length }
@main def run(): Unit = println(E().+:(s("right"))())
