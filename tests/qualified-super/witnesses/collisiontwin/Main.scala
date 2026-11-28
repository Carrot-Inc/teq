class A { def m(): String = "A" }
class B extends A { def `super$m`(): String = "user"; class I { def f = B.this.m() } }
@main def run(): Unit = { val b = new B; println((new b.I).f) }
