package c0786

object Main {
  val x = 10
  def run(): Unit = {
    val y = 10
    trait A { def m(a: Int = y): Int = a + 0 }
    trait B extends A { override def m(a: Int = y + 1): Int = a + 100 }
    trait D extends A { override def m(a: Int): Int = a + 200 }
    class C extends B with D
    println(new C().m())
  }
  def main(args: Array[String]): Unit = run()
}
