import oldsrc.*
import scala.deriving.Mirror

class Mine(val x: Int) extends AnyVal with Odd
class Derived extends Base { def more: Int = size + 1 }

object Main {
  def main(args: Array[String]): Unit = {
    val a: Any = new Wrapped(1)
    val b: Any = new Wrapped(1)
    println(a == b)
    println(a.hashCode)
    println(Set(new Wrapped(1), new Wrapped(1)).size)
    val c: Any = new Mine(1)
    println(c == new Mine(1))
    println(c.hashCode)
    println((new Meter(1) plus new Meter(2)).n)
    val m = summon[Mirror.ProductOf[Pot]]
    println(m.fromProduct(Tuple1(3)))
    println((m: Any) == Pot)
    println(summon[Mirror.Of[Kiln.type]].isInstanceOf[Mirror.Singleton])
    println(new Derived().label() + new Derived().more)
  }
}
