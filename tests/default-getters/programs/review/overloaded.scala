object Main {
  def f(x: Int): Unit = {
    trait T {
      def m(a: Int = x): Int = a + x
      def m(s: String): Int = s.length + x
    }
    class C extends T
    println(new C().m())
  }
  def main(args: Array[String]): Unit = f(10)
}
