transparent inline def id(x: Int): Int = x
class A(val x: Int)
class B(x: Int) extends A(id(x))
class C(override val x: Int) extends B(x)
class G(using val x: Int)
class H(using x: Int) extends G
class I(override val x: Int) extends H(using x)
class H2(using x: Int) extends G(using x)
class I2(override val x: Int) extends H2(using x)
class N(val x: Int, val y: Int)
class M(x: Int, y: Int) extends N(x, y = y)
class O(override val x: Int, override val y: Int) extends M(x, y)
