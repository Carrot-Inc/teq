// A parent's abstract type member in the parameters of an `apply` past 22 parameters, which a child fixes and
// implements as Function23's: the parent's declaration seen from the child (`Type.memberInfo`, `asSeenFrom` of its
// whole information, `Parent.this.T` read as the child's `Int`), its forwarder of `FunctionXXL.apply` decided by
// `allOverriddenSymbols`.
abstract class Parent {
  type T
  def apply(x1: T, x2: T, x3: T, x4: T, x5: T, x6: T, x7: T, x8: T, x9: T, x10: T, x11: T, x12: T, x13: T, x14: T, x15: T, x16: T, x17: T, x18: T, x19: T, x20: T, x21: T, x22: T, x23: T): Int
}
class Child extends Parent with Function23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] {
  type T = Int
  def apply(x1: Int, x2: Int, x3: Int, x4: Int, x5: Int, x6: Int, x7: Int, x8: Int, x9: Int, x10: Int, x11: Int, x12: Int, x13: Int, x14: Int, x15: Int, x16: Int, x17: Int, x18: Int, x19: Int, x20: Int, x21: Int, x22: Int, x23: Int): Int = x1 * 100 + x23
}
object Main {
  def main(args: Array[String]): Unit = {
    val c = new Child
    val f: Function23[Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int] = c
    println(f(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
    val p: Parent { type T = Int } = c
    println(p.apply(2, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 24))
    println(c(3, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 25))
  }
}
