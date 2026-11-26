package c0793

object Main {
  val x = 10
  def run(): Unit = {
    val y = 10
    trait A { def m(a: Int = y): Int = a + 0 }
    trait B extends A { override def m(a: Int = y + 1): Int = a + 100 }
    trait D extends A { override def m(a: Int = y + 2): Int = a + 200 }
    trait E extends B with D { override def m(a: Int = y + 3): Int = a + 300 }
    class C extends E
    println(new C().m())
  }
  def main(args: Array[String]): Unit = run()
}
