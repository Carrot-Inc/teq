// A class extending a function type past 22 parameters gets FunctionXXLForwarders' `apply(Object[])` of the `apply`
// that overrides the function's (`allOverriddenSymbols`), never of another overload of the arity (the pass's probe on
// 39ba4cf2d, where the forwarder called `apply(String, ..)` and failed with a ClassCastException).
package function_xxl_overloaded_apply

class Overloaded extends Function23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] {
  def apply(x1: String, x2: String, x3: String, x4: String, x5: String, x6: String, x7: String, x8: String, x9: String, x10: String, x11: String, x12: String, x13: String, x14: String, x15: String, x16: String, x17: String, x18: String, x19: String, x20: String, x21: String, x22: String, x23: String): Int = -1
  def apply(x1: Int, x2: Int, x3: Int, x4: Int, x5: Int, x6: Int, x7: Int, x8: Int, x9: Int, x10: Int, x11: Int, x12: Int, x13: Int, x14: Int, x15: Int, x16: Int, x17: Int, x18: Int, x19: Int, x20: Int, x21: Int, x22: Int, x23: Int): Int = x1 + x23
}

@main def main(): Unit =
  val f: Function23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = new Overloaded
  println(f(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
  val o = new Overloaded
  println(o(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
