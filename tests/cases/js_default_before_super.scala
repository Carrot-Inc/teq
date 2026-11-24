// A parent call whose arguments are hoisted ahead of the parent's constructor (a default before a written argument,
// or named arguments out of order) in a class nested in another: the hoisted default getter and a hoisted written
// argument that reads the enclosing instance read it from the constructor's own parameter, which JavaScript allows
// before `super`; an enum case of a member enum reads it before the case's constructor sets its field.
object V { def n(k: Int): Int = { println("arg " + k); k } }

class LC {
  val base: Int = 500
  val q: Int = 5
  def m: Int = { println("m"); 7 }
  class C(val x: Int, val y: Int = base + 40, val z: Int = 2) { def s: Int = x * 100 + y * 10 + z }
  object K extends C(z = V.n(3), x = V.n(1))
  class KC extends C(x = V.n(1), z = V.n(3))
  object W extends C(z = V.n(9), x = q + 1)
  class WM extends C(z = V.n(9), y = 4, x = m)
  class D { val r: Int = 3; object K2 extends C(z = V.n(9), x = q + r) }
  class S(val x: Int, val y: Int, val z: Int) { def this(s: String, y: Int = base, z: Int) = this(s.length, y, z); def t: Int = x + y + z }
  object KS extends S(z = V.n(3), s = "four")
  enum E(val x: Int, val y: Int = 1, val z: Int = 2) { def s: Int = x * 100 + y * 10 + z; case EK extends E(z = V.n(9), x = 7) }
}

trait Tr {
  val q: Int = 6
  class C(val x: Int, val y: Int = q * 10, val z: Int = 2) { def s: Int = x * 100 + y * 10 + z }
  object K extends C(z = V.n(4), x = q)
}
class TrI extends Tr

@main def main(): Unit =
  val lc = new LC
  println(lc.K.s)
  println(new lc.KC().s)
  println(lc.W.s)
  println(new lc.WM().s)
  println(new lc.D().K2.s)
  println(lc.KS.t)
  println(lc.E.EK.s)
  println(new TrI().K.s)
  def local(): Int =
    class L { val b: Int = 30; class C(val x: Int = b, val y: Int); object K extends C(y = V.n(2)); def s: Int = K.x + K.y }
    new L().s
  println(local())
