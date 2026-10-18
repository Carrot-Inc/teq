class A(using val x: Int)
class C extends A(using 1) { override val x: Int = 2 }
class B(using x: Int) extends A
class D(override val x: Int) extends B(using x)
