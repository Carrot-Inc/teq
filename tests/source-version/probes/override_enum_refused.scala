class A(val s: Int)
enum F(s: Int) extends A(s + 1):
  case G(override val s: Int) extends F(s)
trait T(val t: Int)
enum H extends T(1):
  case K(override val t: Int)
class D(val u: Int = 0)
trait U(v: Int) extends D
class V(u: Int) extends U(u)
class W(override val u: Int) extends V(u)
