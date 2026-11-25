// A right-associative extension written out with a receiver default, a leading and a middle using clause, a named
// later argument and a further clause: the prefix, then each clause as written, the defaults in their clauses' turn.
def n(s: String): Int = { println(s); 1 }
def s(s: String): String = { println(s); s }
class E:
  println("E.init")
  extension (using lead: Boolean)(l: Int = n("D-left"))(using mid: Double)
    def +:(r: String)(a: Int = n("D-a"), b: Int = n("D-b"))(z: Int): Int = { println("body"); l + a + b + z }
def e(): E = { println("prefix"); new E }
def b(): Boolean = { println("leading-using"); true }
def d(): Double = { println("middle-using"); 1.0 }
@main def run(): Unit = println(e().+:(using b())(s("right"))()(using d())(b = n("b"))(n("last")))
