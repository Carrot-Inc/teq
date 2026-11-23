// teq: --wconf msg=Non.local.returns.are.no.longer.supported:s
// A `return` in an inline method's by-name argument is in the argument's code, a def of its own in the
// expansion (dotty binds it to a proxy def), so its call is no loop's jump; one in an `inline` lambda argument is
// the method's own code once inlined, and goes round the loop a hundred thousand deep.
object X {
  inline def twice(x: => Int): Int = x + x
  inline def app(inline g: Int => Int, v: Int): Int = g(v)
  final def viaInlineByName(n: Int): Int = twice { if n > 0 then return viaInlineByName(n - 1); 1 }
  final def viaInlineLambda(n: Int): Int = app(k => { if k > 0 then return viaInlineLambda(k - 1); 3 }, n)
}
object Main {
  def main(args: Array[String]): Unit = {
    println(X.viaInlineByName(3))
    println(X.viaInlineLambda(3))
    println(X.viaInlineLambda(100000))
  }
}
