// A local trait's default and the block's values (the design read of 24 Nov, its probe InlineCapture): an inline member whose default reads a capture (scalac 12; master refuses whole, its own products read right).
object Lib {
  def value(x: Int): Int = {
    trait T { inline def m(a: Int, b: Int = x): Int = a + b }
    class C extends T
    new C().m(2)
  }
}
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
