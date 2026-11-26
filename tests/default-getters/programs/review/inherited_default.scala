object Main {
  def f(x: Int): Unit = {
    trait A { def m(a: Int = 1): Int = x + a }
    trait B extends A {
      override def m(a: Int = 2): Int = x + a
    }
    class C extends A
    class D extends C with B
    println(new D().m())
  }
  def main(args: Array[String]): Unit = f(10)
}
