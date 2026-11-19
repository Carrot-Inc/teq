// An overriding final val whose Int literal a union's numeric member widens (`Double | String`) takes the view's
// result, a Double: its getter is the Double's, bridged to the union's erasure, read through the parent alike, from a
// class and from an object mixing a trait in.
trait Base { def x: Double | String }
class C extends Base { override final val x = 1 }
trait T extends Base { override final val x = 2 }
object O extends T
@main def main(): Unit =
  val c = new C
  val b: Base = c
  println(c.x + 0.5)
  println(b.x == 1.0)
  println(O.x / 4)
  val b2: Base = O
  println(b2.x == 2.0)
