object Main {
var y = 0; trait A { def m(a: Int = { y += 1; y }): Int = a }; class C extends A; def run(): Unit = println(new C().m())
def main(args: Array[String]): Unit = { run(); run() }
}
