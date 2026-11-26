object Main {
  def f(x: Int): Unit = {
    trait A {
      def m(a: Int = { println("A-default"); x }): Int = a
    }
    trait B extends A {
      override def m(
        a: Int = { println("B-default"); x + 2 }
      ): Int = {
        println("B-m")
        a * 2
      }
    }
    class C extends A with B
    println(new C().m())
  }
  def main(args: Array[String]): Unit = f(10)
}
