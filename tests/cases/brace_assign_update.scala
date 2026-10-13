// A brace body whose first statement, on the `{`'s line, is a method ending in an assignment
// (`a(i) = v`): that line's break ends it before the next member, and `c(0) = 3` is the
// `update` call (dotty's `Scanners.handleNewLine`, `Desugar` of an application's assignment).
class C(a: Array[Int]) { def update(i: Int, v: Int): Unit = a(i) = v
  def apply(i: Int): Int = a(i)
}
@main def run(): Unit =
  val c = new C(Array(1))
  c(0) = 3
  println(c(0))
