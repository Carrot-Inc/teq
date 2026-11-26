object Main {
def run(): Unit = {
val y = 8;
trait A { def m(a: Int)(b: Int = a + y): Int = b }; class C extends A
println(new C().m(2)())
}
def main(args: Array[String]): Unit = run()
}
