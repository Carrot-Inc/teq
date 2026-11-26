object Main {
val y = 8;
trait A { def m(a: Int)(b: Int = a + y): Int = b }; class C extends A
def run(): Unit = { println(new C().m(2)()) }
def main(args: Array[String]): Unit = run()
}
