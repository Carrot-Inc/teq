package p {
  class A(private[p] val s: Int)
  class B(s: Int) extends A(s)
  class Inside(override val s: Int) extends B(s)
  class PA(private[p] val s: Int)
  class PB(override private[p] val s: Int) extends PA(s)
  class PC(override val s: Int) extends PB(s)
  enum F(private[p] val s: Int):
    case G(override val s: Int) extends F(s)
}
class C(override val s: Int) extends p.B(s)
class Outer:
  class OA(private[Outer] val s: Int)
  def local(): Int =
    class LA(private[Outer] val s: Int)
    class LC(override val s: Int) extends LA(s)
    enum LE(private[Outer] val s: Int):
      case LG(override val s: Int) extends LE(s)
    LC(1).s + LE.LG(2).s
object Outer:
  val o = new Outer
  class OC(override val s: Int) extends o.OA(s)
object Other:
  val o = new Outer
  class OD(override val s: Int) extends o.OA(s)
class Prot(protected val s: Int)
class ProtC(override val s: Int) extends Prot(s)
enum E(s: Int) extends p.A(s):
  case K(override val s: Int) extends E(s)
