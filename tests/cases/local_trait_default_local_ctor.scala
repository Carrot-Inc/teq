// A local trait's default and the block's values (the design read of 24 Nov, its probe LocalCtor): a local class's secondary constructor whose default reads a capture (a control: right on master).
object Lib {
  def value(x: Int): Int = {
    class C(val n: Int) { def this(s: String, n: Int = x) = this(n); def result = x + n }
    new C("hi").result
  }
}
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
