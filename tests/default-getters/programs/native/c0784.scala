package c0784

object Main {
  val x = 10
  def run(): Unit = {
    val y = 10
    trait A { def m(a: Int = y): Int = a + 0 }
    trait B extends A { override def m(a: Int): Int = a + 100 }
    abstract class P extends A
    class C extends P with B
    println(new C().m())
  }
  def main(args: Array[String]): Unit = run()
}
