package ilocal
class A { def m(): String = "A" }
object Lib { inline def f: String = { class B extends A { class I { def g = B.super.m() }; def g = (new I).g }; (new B).g }; class I { def g = f } }
@main def run(): Unit = { println((new Lib.I).g); println(Lib.f) }
