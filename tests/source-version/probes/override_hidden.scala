class A(val x: Int)
class B(override val x: Int) extends A(x)
class C(override val x: Int) extends B(x)
class G(y: Int) extends C(y) { override val x = 2 }
