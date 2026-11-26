object Main {
val y = 10;
trait A { def m(a: Int = y): Int = a }; trait B extends A { override def m(a: Int = y + 1): Int = a }; trait D extends A { override def m(a: Int = y + 2): Int = a }; class C extends B with D
def run(): Unit = { println(new C().m()) }
def main(args: Array[String]): Unit = run()
}
