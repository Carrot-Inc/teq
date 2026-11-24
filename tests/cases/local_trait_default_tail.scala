// A local trait's default and the block's values (the design read of 24 Nov, its probe Tail): a tail-recursive member of a capturing local trait, every argument written (a control: right on master).
object Lib {
  def value(x: Int): Int = {
    trait T { final def m(a: Int, b: Int = 7): Int = if (a == 0) x + b else m(a - 1, b) }
    class C extends T
    new C().m(2, 3)
  }
}
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
