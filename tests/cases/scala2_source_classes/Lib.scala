// The classes of a source under `import scala.language.`2.13`` are Scala 2's to the program that holds them, as
// scalac 3.8.4 marks them where it enters them: a value class below a trait that overrides `equals` and `hashCode`
// has the rule's two all the same, an alphanumeric method is an operator without a warning, a case class's mirror is
// not its companion and a case object's is no `Mirror.Singleton`.
package oldsrc
import scala.language.`2.13`

trait Odd extends Any {
  override def equals(o: Any): Boolean = false
  override def hashCode: Int = 7
}
class Wrapped(val x: Int) extends AnyVal with Odd

class Meter(val n: Int) { def plus(o: Meter): Meter = new Meter(n + o.n) }

case class Pot(glaze: Int)
case object Kiln

class Base { def label(): String = "base"; def size: Int = 1 }
