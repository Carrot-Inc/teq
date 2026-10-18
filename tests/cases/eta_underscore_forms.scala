// `f _` as dotty's `Migrations.typedAsFunction` expands it: a method of each arity, of `()` (an expansion, not a
// call), curried, vararg, a function value left as it is, the prefix evaluated once where the expansion stands. The
// migration warning of the trailing `_` (from 3.4) is silenced.
// teq: --wconf msg=eta-expansion.is.unnecessary:s
class C { def f(): Int = { println("f"); 1 }; def g(x: Int): Int = x }

object P {
  var made = 0
  def mk(): C = { made += 1; new C }
  def nullary(): Int = 1
  def one(x: Int): Int = x + 1
  def curried(x: Int)(y: Int): Int = x + y
  def vararg(xs: Int*): Int = xs.sum
  val k: Int => Int = x => x * 10

  @annotation.nowarn("msg=eta-expansion")
  def main(args: Array[String]): Unit =
    val a = nullary _
    val b = one _
    val c = curried _
    val d = curried(1) _
    val e = vararg _
    val m = k _
    println((a(), b(2), c(1)(2), d(3), e(Seq(1, 2)), m(4)))
    val f = mk().f _
    val g = mk().g _
    println(made)
    println(f() + f() + g(2))
    println(made)
}
