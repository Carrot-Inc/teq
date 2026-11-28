class A { def m(): String = "A" }
class B extends A { def f = () => super.m(); def g = () => B.super.m() }
object O extends A { def f = () => super.m(); def g = () => O.super.m() }
trait T extends A { def f = () => super.m(); def g = () => T.super.m() }
class C extends A with T
@main def run(): Unit = { val b = new B; val c = new C; println(b.f()); println(b.g()); println(O.f()); println(O.g()); println(c.f()); println(c.g()) }
