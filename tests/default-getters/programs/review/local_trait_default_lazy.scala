// A local trait's default and the block's values (the design read of 24 Nov, its probe Lazy): a lazy val of a capturing local trait beside a default (a control: right on master).
object Lib {
  def value(x: Int): Int = {
    trait T { lazy val y: Int = x; def m(a: Int = 7): Int = y + a }
    new T {}.m()
  }
}
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
