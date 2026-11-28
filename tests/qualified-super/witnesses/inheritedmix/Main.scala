class A { def m(): String = "A" }
trait U extends A
class B extends A with U { class I { def f = B.super[U].m() } }
@main def run(): Unit = { val b = new B; println((new b.I).f) }
