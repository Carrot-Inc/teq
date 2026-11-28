class A { def m(): String = "A" }
class Mid extends A { override def m() = "Mid" }
trait T extends A { def f = T.super[A].m() }
class C extends Mid with T
@main def run(): Unit = println((new C).f)
