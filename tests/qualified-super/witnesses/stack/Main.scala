package stack
class A { def m(): String = "A" }
trait T extends A { class I { def f = T.super.m() } }
trait U extends A { override def m() = "U" }
trait V extends A { override def m() = "V" }
class Mid extends A { override def m() = "Mid" }
class C1 extends A with U with V with T
class C2 extends A with V with U with T
class C3 extends Mid with T
object O extends Mid with T
trait Pin extends A { class J { def f = Pin.super[A].m() } }
class C4 extends Mid with Pin
@main def run(): Unit = {
 val a = new C1; val b = new C2; val c = new C3; val d = new Mid with T; val e = new C4
 println((new a.I).f); println((new b.I).f); println((new c.I).f); println((new O.I).f); println((new d.I).f); println((new e.J).f)
}
