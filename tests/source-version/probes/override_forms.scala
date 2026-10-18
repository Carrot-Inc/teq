class A(val x: Int)
class B extends A(1) { override val x = 2 }
class C(override val x: Int) extends A(x)
class D extends C(3)
class E(x: Int) extends A(x)
class F(override val x: Int) extends E(x)
class G(override val x: Int) extends A(5)
trait T { val y: Int }
class H(val y: Int) extends T
class I extends H(1) { override val y = 4 }
