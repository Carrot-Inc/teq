// A constructor's default of an earlier clause is evaluated before a later clause's arguments
// written out of order (`TypedApply.result`'s lifting, `liftFun` of the constructor call), and
// within a clause the explicit arguments run in the order written, the defaults after them.
object CtorPlan:
  def mark(s: String): Int = { print(s); 1 }
  class C(a: Int = mark("A"), b: Int)(c: Int, d: Int)
  case class P(x: Int = mark("x"), y: Int = mark("y"), z: Int = mark("z"))

  def main(args: Array[String]): Unit =
    new C(b = 0)(d = mark("D"), c = mark("C")); println()
    new C(b = mark("b"))(mark("c"), mark("d")); println()
    P(z = mark("Z"), y = mark("Y")); println()
    P(y = mark("Y")); println()
