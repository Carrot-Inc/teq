object Main {
val y = 10;
trait A { def m(a: Int = 100 + y): Int = a * 10 + 1 }; class C extends A { override def m(a: Int = 300 + y): Int = a * 10 + 3 }
def run(): Unit = { println((new C(): A).m()) }
def main(args: Array[String]): Unit = run()
}
