import oldsrc.*

class Mine(val x: Int) extends AnyVal with Odd
class Derived extends Base { def more: Int = size + 1 }

object Main {
  def main(args: Array[String]): Unit = {
    val a: Any = new Wrapped(1)
    val b: Any = new Wrapped(1)
    println(a == b)
    println(Set(new Wrapped(1), new Wrapped(1)).size)
    val c: Any = new Mine(1)
    println(c == new Mine(1))
    println((new Meter(1) plus new Meter(2)).n)
    println(new Derived().label() + new Derived().more)
  }
}
