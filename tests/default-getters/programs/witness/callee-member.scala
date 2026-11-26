object Main {
trait A[T] { def m(a: T = null.asInstanceOf[T]): String = "A" + a }; trait B extends A[String] { override def m(a: String = "b"): String = "B" + a }; trait D[T] extends A[T] { override def m(a: T = "d".asInstanceOf[T]): String = "D" + a }; class C extends B with D[String]
def run(): Unit = { println(new C().m()); println((new C(): A[String]).m()); println((new C(): B).m()) }
def main(args: Array[String]): Unit = run()
}
