object Main {
class C { def m[A](a: A = "yes"): A = a }; def run(): Unit = { println(new C().m()) }
def main(args: Array[String]): Unit = run()
}
