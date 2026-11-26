object Main {
def run(): Unit = {
trait A { def m(a: Int = 10): Int = a }; trait B extends A { override def m(a: Int = 20): Int = a + 100 }; class C extends B { override def m(a: Int = 30): Int = a + 200; def f(): Int = super.m(); def g(): Int = super[B].m() }
println(new C().f()); println(new C().g()); println((new C(): A).m())
}
def main(args: Array[String]): Unit = run()
}
