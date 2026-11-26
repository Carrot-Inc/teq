object Main {
def run(): Unit = {
trait A { def m(a: Int = y): Int = a }; class C extends A; val result = new C().m(); val y = 10
println(result)
}
def main(args: Array[String]): Unit = run()
}
