package p {
  class A(private[p] val s: Int)
  class B(override private[p] val s: Int) extends A(s)
}
class C(override val s: Int) extends p.B(s)
