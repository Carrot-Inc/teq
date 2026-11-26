object Main {
def run(): Unit = { class C { def m[A](a: A = "yes"): A = a }; println(new C().m()) }
def main(args: Array[String]): Unit = run()
}
