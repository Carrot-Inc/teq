class A { def m(x: String = "A"): String = x }
class B extends A { class I { def f = B.super.m() } }
@main def run(): Unit = { val b = new B; println((new b.I).f) }
