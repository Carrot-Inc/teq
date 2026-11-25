// A right-associative extension member reached through super[T], partially applied with `_`, and selected as a
// function: each takes its clauses in the compiled order.
def n(s: String): Int = { println(s); 7 }
def s(): String = { println("right"); "r" }
trait T:
  extension (l: Int)
    def +:(r: String): Int = { println("T.body"); l }
class C extends T:
  def test(): Int = super[T].+:(s())(n("left"))
class E:
  extension (l: Int)
    def +:(r: String): Int = { println("E.body"); l }
def e(): E = { println("prefix"); new E }
@main def run(): Unit =
  println((new C).test())
  val f: Int => Int = e().+:(s()) _
  println("function-created")
  println(f(n("left")))
  val q = e()
  import q.*
  val g: Int => Int = s().+:
  println("selection-created")
  println(g(n("left")))
