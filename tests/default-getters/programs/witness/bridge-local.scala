object Main {
def run(): Unit = {
trait A[T] { def m(a: T = null.asInstanceOf[T]): String = "A" + a }; trait B extends A[String] { override def m(a: String = "b"): String = "B" + a }; class C extends B
println(new C().m()); println((new C(): A[String]).m())
}
def main(args: Array[String]): Unit = run()
}
