// A right-associative extension's later clause given by name, its other parameter's default left out: the explicit
// argument runs before the default.
def n(s: String): Int = { println(s); 1 }
def s(s: String): String = { println(s); s }
class E:
  extension (l: Int)
    def +:(r: String)(a: Int = n("D-a"), b: Int = n("D-b")): Int = { println("body"); l + a + b }
@main def run(): Unit = println(E().+:(s("right"))(n("left"))(b = n("b")))
