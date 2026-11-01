class A()(implicit val s: Int)
class B()(implicit s: Int) extends A()(using s + 1)
class C()(implicit override val s: Int) extends B()
