// A local trait's default and the block's values (the design read of 24 Nov, its probe SuperPrior): an override's super call leaves a second clause's default reading the first (scalac 21; master: the getter missing).
object Lib {
  def value(x: Int): Int = {
    trait T { def m(a: Int)(b: Int = a + 7): Int = x + a + b }
    class C extends T { override def m(a: Int)(b: Int): Int = super.m(a)() }
    new C().m(2)(3)
  }
}
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
