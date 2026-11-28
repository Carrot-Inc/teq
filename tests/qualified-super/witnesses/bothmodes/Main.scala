package bothmodes
class A { def m(): String = "A" }
class Mid extends A { override def m() = "Mid" }
trait Both extends A { class I { def f = Both.super.m() + "," + Both.super[A].m() } }
class C extends Mid with Both
@main def run(): Unit = { val c = new C; println((new c.I).f) }
