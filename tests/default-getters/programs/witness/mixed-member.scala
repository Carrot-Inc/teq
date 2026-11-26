trait A { def m(a: Int = 10): Int = a }
object Main {
val y = 20;
trait B extends A { override def m(a: Int = y): Int = a + 100 }; class C extends B
def run(): Unit = { println((new C(): A).m()); println(new C().m()) }
def main(args: Array[String]): Unit = run()
}
