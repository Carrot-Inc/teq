object Main {
val y = 10;
trait A { def m(a: Int = y): Int = a }; trait B extends A { override def m(a: Int): Int = a + 100 }; class C extends B
def run(): Unit = { println(new C().m()) }
def main(args: Array[String]): Unit = run()
}
