object Main {
def run(): Unit = {
trait A { def m(a: Int = 10): Int = a; def m(s: String): Int = s.length }; trait B extends A { override def m(a: Int): Int = a + 100 }; class C extends B
println(new C().m())
}
def main(args: Array[String]): Unit = run()
}
