// A program that waits for dotty's TypeErasure to be ported
// whole (the signature of deeply nested arrays): an `apply`
// past 22 parameters whose first parameter nests 18 arrays,
// beside a base's overload that differs from it in that
// parameter's element type alone, the class a Function23
// too. dotty tells the base's two alternatives apart by
// the parameter's signature name (`TypeErasure.sigName`'s
// `JavaArrayType` arm, `scala.Int` with 18 `[]`) in
// `atSignature` and the double definition check. The case
// moves to tests/cases with the change that makes it pass.
object Deep {
  type I18 = Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Int]]]]]]]]]]]]]]]]]]
  type S18 = Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[String]]]]]]]]]]]]]]]]]]
  trait Base {
    def apply(a: I18, x2: Int, x3: Int, x4: Int, x5: Int, x6: Int, x7: Int, x8: Int, x9: Int, x10: Int, x11: Int, x12: Int, x13: Int, x14: Int, x15: Int, x16: Int, x17: Int, x18: Int, x19: Int, x20: Int, x21: Int, x22: Int, x23: Int): Int
    def apply(a: S18, x2: Int, x3: Int, x4: Int, x5: Int, x6: Int, x7: Int, x8: Int, x9: Int, x10: Int, x11: Int, x12: Int, x13: Int, x14: Int, x15: Int, x16: Int, x17: Int, x18: Int, x19: Int, x20: Int, x21: Int, x22: Int, x23: Int): Int
  }
  class C extends Base with Function23[I18, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] {
    def apply(a: I18, x2: Int, x3: Int, x4: Int, x5: Int, x6: Int, x7: Int, x8: Int, x9: Int, x10: Int, x11: Int, x12: Int, x13: Int, x14: Int, x15: Int, x16: Int, x17: Int, x18: Int, x19: Int, x20: Int, x21: Int, x22: Int, x23: Int): Int = a.length * 100 + x2 + x23
    def apply(a: S18, x2: Int, x3: Int, x4: Int, x5: Int, x6: Int, x7: Int, x8: Int, x9: Int, x10: Int, x11: Int, x12: Int, x13: Int, x14: Int, x15: Int, x16: Int, x17: Int, x18: Int, x19: Int, x20: Int, x21: Int, x22: Int, x23: Int): Int = -a.length
  }
  def main(args: Array[String]): Unit = {
    val i: I18 = Array.fill(3)(null)
    val s: S18 = Array.fill(5)(null)
    val c = new C
    val f: Function23[I18, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = c
    println(f(i, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
    println((c: Base)(s, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
    println((c: Base)(i, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
  }
}
