package r0111

object Main {
  def run(): Unit = {
    trait A { def m(a: Int): Int = a + 0; def m(a: Int, b: Int): Int = a + b + 0 }
    trait B extends A { override def m(a: Int): Int = a + 100 }
    trait D extends A { override def m(a: Int): Int = a + 200 }
    class C extends B with D
    println(new C().m(10))
    println(new C().m(10, 1))
  }
  def main(args: Array[String]): Unit = run()
}
