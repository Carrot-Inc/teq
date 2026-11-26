// A local trait's default and the block's values (the design read of 24 Nov, its probe EscapePrior): an instance made in a method, its default left through a non-local interface by another object (scalac 22; master JavaScript 21).
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(x: Int): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + 8): Int = x + a + b }
    class C extends T { override def m(a: Int)(b: Int): Int = super.m(a)(b) }
    new C
  }
}
object Caller { def call(t: API): Int = t.m(2)() }
object Main { def main(args: Array[String]): Unit = println(Caller.call(Lib.make(10))) }
