package r0117

object Main {
  def run(): Unit = {
    trait A { def m(a: Int): Int; def m(s: String): Int = s.length + 0 }
    trait B extends A { override def m(a: Int): Int = a + 100 }
    trait D extends A { override def m(a: Int): Int = a + 200 }
    class C extends B with D
    println(new C().m(10))
    println(new C().m("ab"))
  }
  def main(args: Array[String]): Unit = run()
}
