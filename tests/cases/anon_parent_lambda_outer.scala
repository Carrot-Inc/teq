// An anonymous class whose parent's arguments are hoisted ahead of the parent's constructor and include a lambda
// reading the enclosing instance: teq runs it on every target, printing what Scala.js prints, where the class scalac
// 3.8.4 writes for the JVM does not load (java.lang.VerifyError: "Type uninitializedThis ... is not assignable");
// docs/COMPATIBILITY.md, "Known differences".
object V { def n(k: Int): Int = { println("arg " + k); k } }

class O {
  val q: Int = 5
  class C(f: Int => Int, val y: Int = 1, val z: Int = 2) { def x: Int = f(0); def s: Int = x * 100 + y * 10 + z }
  def reordered: Int = new C(z = V.n(9), f = k => k + q) {}.s
  def inOrder: Int = new C(f = k => k + q, z = V.n(9)) {}.s
  def full: Int = new C(z = V.n(9), y = V.n(4), f = k => k + q) {}.s
}

trait Tr {
  val q: Int = 6
  class C(f: Int => Int, val y: Int = 1, val z: Int = 2) { def s: Int = f(0) * 100 + y * 10 + z }
  def reordered: Int = new C(z = V.n(9), f = k => k + q) {}.s
}
class TrI extends Tr

@main def main(): Unit =
  val o = new O
  println(o.reordered)
  println(o.inOrder)
  println(o.full)
  println(new TrI().reordered)
  def local(): Int =
    class L { val q: Int = 7; class C(f: Int => Int, val z: Int = 2) { def s: Int = f(0) * 10 + z }; def s: Int = new C(z = V.n(1), f = k => k + q) {}.s }
    new L().s
  println(local())
