// A local trait's default and the block's values (the design read of 24 Nov, its probe Clause): a second clause's default reading the first, left on a class mixing the trait in (scalac 21; master: the local a out of scope).
object Lib {
  def value(x: Int): Int = {
    trait T { def m(a: Int)(b: Int = a + 7): Int = x + a + b }
    class C extends T
    new C().m(2)()
  }
}
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
