class A { inline def m(): String = "A" }
class B extends A { class I { def f = B.super.m() } }
@main def run(): Unit = { val b = new B; println((new b.I).f) }
