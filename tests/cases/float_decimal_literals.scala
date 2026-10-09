// An unsuffixed decimal is typed from its digits against the expected type (dotty's
// `Parsers.literal` and `Typer.typedNumber`): a `Float` read from the digits themselves where
// one is expected, so `1.0000000596046448` is the nearest `Float` to the number written, not the
// `Float` nearest to the `Double` nearest to it; a `Double` anywhere else, a tuple's element and
// an argument picking an overload among them.
type F = Float

def f(x: Float): Float = x

def h(x: Float): String = "float"
def h(x: Double): String = "double"

def k(x: Float, y: Int): Boolean = x == 1.1f
def k(x: String): Boolean = false

@main def run(): Unit =
  val direct: Float = 1.0000000596046448
  val viaDouble: Float = 1.0000000596046448.toFloat
  println(direct == viaDouble)
  println(direct == 1.0000001f)
  val a: Float = -1.5
  val b: F = 2.25
  val c: Float = if a < 0 then 1.0e3 else .5
  val d: Float = { println("block"); 3.5 }
  val g = List[Float](1.5, 0.3)
  val i: Float = 1_000.5
  val j: Float = -0e5
  val l: Float = 1.5e-3
  println(List(a == -1.5f, b == 2.25f, c == 1000f, d == 3.5f, f(0.1) == 0.1f, g == List(1.5f, 0.3f), i == 1000.5f, 1 / j == Float.NegativeInfinity, l == 0.0015f))
  println(Array[Float](1.1, 2.5)(0) == 1.1f)
  println(h(1.5))
  println(k(1.1, 1))
  val x: Float = 1.1f
  x match
    case 1.1 => println("float pattern")
    case _ => println("other")
  val any: Any = 1.1
  any match
    case 1.1 => println("double pattern")
    case _ => println("other")
