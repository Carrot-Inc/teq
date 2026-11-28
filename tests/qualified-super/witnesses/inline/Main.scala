class A { def m(): String = "A" }
class B extends A { inline def f: String = super.m(); class I { def g = f } }
@main def run(): Unit = { val b = new B; println((new b.I).g) }
