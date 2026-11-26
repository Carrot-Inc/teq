object Main {
  def main(args: Array[String]): Unit = {
    var x = 10
    trait T {
      def m(a: Int = { println("default"); x }): Int = {
        println("m")
        a
      }
    }
    class C extends T
    val c = new C
    println(c.m())
    x = 20
    println(c.m())
  }
}
