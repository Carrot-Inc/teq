// The parameters of a synthetic function class's `apply`, which `syntheticParamNames` names `x$0` on
// (Definitions.newFunctionNType), passed by name; a function class spelled otherwise than its arity is a class of its own.
object Main {
  def test(f: scala.Function023[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int]): Int = f.apply(x$0 = 1, x$1 = 2, x$2 = 3, x$3 = 4, x$4 = 5, x$5 = 6, x$6 = 7, x$7 = 8, x$8 = 9, x$9 = 10, x$10 = 11, x$11 = 12, x$12 = 13, x$13 = 14, x$14 = 15, x$15 = 16, x$16 = 17, x$17 = 18, x$18 = 19, x$19 = 20, x$20 = 21, x$21 = 22, x$22 = 23)
  def main(args: Array[String]): Unit = {
    val f = new scala.Function023[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] {
      def apply(a1: Int, a2: Int, a3: Int, a4: Int, a5: Int, a6: Int, a7: Int, a8: Int, a9: Int, a10: Int, a11: Int, a12: Int, a13: Int, a14: Int, a15: Int, a16: Int, a17: Int, a18: Int, a19: Int, a20: Int, a21: Int, a22: Int, a23: Int): Int = a1 * 100 + a23
    }
    println(test(f))
    val g: scala.ContextFunction1[Int, Int] = x ?=> x + 1
    println(g.apply(using x$0 = 41))
  }
}
