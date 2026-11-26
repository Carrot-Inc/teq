package r0123

object Main {
  def run(): Unit = {
    trait A { def m(a: Int): Int = a + 0; def m(s: String): Int = s.length + 0 }
    trait B extends A { override def m(a: Int): Int = a + 100 }
    trait D extends A { override def m(a: Int): Int = a + 200 }
    trait E extends B { override def m(a: Int): Int = a + 300 }
    class C extends E with D with B
    println(new C().m(10))
    println(new C().m("ab"))
  }
  def main(args: Array[String]): Unit = run()
}
