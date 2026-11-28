class A { def m(): String = "A" }
class B extends A { class I { def f = B.super.m() }; override def m() = "B" }
class C extends B { class J { def f = C.super.m() } }
@main def run(): Unit = { val c = new C; println((new c.I).f); println((new c.J).f) }
