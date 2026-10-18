class A(val x: Int)
class B(override val x: Int) extends A(x)
class C(override val x: Int) extends B(x)
class D(x: Int) extends B(x)
class E(override val x: Int) extends D(x)
class F(override val x: Int) extends B(1)
class G(y: Int) extends A(y) { override val x = 2 }
