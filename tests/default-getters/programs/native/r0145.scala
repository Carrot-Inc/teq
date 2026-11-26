package r0145

object Main {
  def run(): Unit = {
    trait A[T] { def m(a: T): Int = a.asInstanceOf[Int] + 0; def m(s: String): Int = s.length + 0 }
    trait B extends A[Int] { override def m(a: Int): Int = a + 100 }
    trait D extends A[Int] { override def m(a: Int): Int = a + 200 }
    trait E extends B with D
    class C extends E
    println(new C().m(10))
    println(new C().m("ab"))
  }
  def main(args: Array[String]): Unit = run()
}
