trait A { def m: String }
trait T extends A { abstract override def m: String = "T"; class I { def f = T.super.m } }
class C extends A { val m = "C" }
class D extends C with T
@main def run(): Unit = { val d = new D; println((new d.I).f) }
