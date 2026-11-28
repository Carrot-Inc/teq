trait A { def m: String = "A" }
trait T extends A { class I { def f = T.super.m } }
trait V extends A { override val m: String = "V" }
class D extends V with T
@main def run(): Unit = { val d = new D; println((new d.I).f) }
