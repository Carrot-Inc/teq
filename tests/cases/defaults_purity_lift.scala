// What the plan lifts follows `TreeInfo`'s purity: a block's immutable val caps it at `Pure`
// (`statPurity`), a lazy val's read is idempotent, a var in a block impure, and a reference reads
// as written: an export forwarder is a stable member, a pure path.
object Shapes {
  def touch(): Int = { print("M"); 1 }
  def sum(a: Int, b: Int = 2): Int = a + b
  object Defaults { val unit = touch() }
}
object Everything { export Shapes.* }
object Probe {
  def mark(s: String): Int = { print(s); 1 }
  def f(a: Int)(b: Int = mark("B")): Int = a + b
  def g(a: Int, b: Int): Int = a + b
  def h(a: Int = mark("D"))(b: Int): Int = a + b
  def run(): Unit = {
    lazy val id = mark("L")
    val path = 2
    println(f(id)())
    println(f(path)())
    println(f({ val x = path; x })())
    println(g(b = mark("Z"), a = { var q = path; q }))
    println(h({ var a = mark("A"); a })(mark("B")))
  }
  def exported(): Unit = {
    import Everything.*
    println(sum(b = Defaults.unit, a = 1))
    println(Everything.sum(b = Everything.Defaults.unit, a = 2))
  }
}
@main def main(): Unit = { Probe.run(); Probe.exported() }
