package oldsrc
import scala.language.`2.13`

trait Odd extends Any {
  override def equals(o: Any): Boolean = false
  override def hashCode: Int = 7
}
class Wrapped(val x: Int) extends AnyVal with Odd

class Meter(val n: Int) { def plus(o: Meter): Meter = new Meter(n + o.n) }

class Base { def label(): String = "base"; def size: Int = 1 }
