// Vals, vars, lazy vals, defaults, by-name, varargs, overloads, type members, givens,
// extensions, opaque types, exports (from an object: teq exports from objects and packages
// alone), a trait calling super, a sealed hierarchy.
package probe.members

trait Base:
  def hello(x: Int): String = "base"
trait Loud extends Base:
  override def hello(x: Int): String = super.hello(x) + "!"
abstract class Holder[T](init: T):
  val fixed: T = init
  var mutable: Int = 0
  lazy val lzy: String = "lazy"
  def withDefault(a: Int, b: Int = 2): Int = a + b
  def byName(x: => Int): Int = x
  def varargs(xs: Int*): Int = xs.sum
  def over(x: Int): Int = x
  def over(x: String): String = x
  def poly[A <: AnyRef](a: A): A = a
  type Member
  type Bounded <: CharSequence
  type Fixed = List[T]
  protected def prot: Int = 1
  private def priv: Int = 2
  private[members] def scoped: Int = 3
object Givens:
  given intShow: Show[Int] with
    def show(a: Int): String = a.toString
  given Show[String] = (s: String) => s
  given listShow[A](using s: Show[A]): Show[List[A]] with
    def show(as: List[A]): String = as.map(s.show).mkString
  implicit val legacy: Ordering[Int] = Ordering.Int
trait Show[A]:
  def show(a: A): String
object Exts:
  extension (s: String)
    def shout: String = s.toUpperCase
    def repeat(n: Int): String = s * n
  extension [A](xs: List[A]) def second: A = xs(1)
object Opaques:
  opaque type Meters = Double
  object Meters:
    def apply(d: Double): Meters = d
  extension (m: Meters) def value: Double = m
class Exporter:
  export Impl.{run, name as label}
object Impl:
  def run(x: Int): Int = x
  def name: String = "impl"
sealed trait Shape
final case class Circle(r: Double) extends Shape
case object Dot extends Shape
class Square(val side: Double) extends Shape
class Value(val underlying: Int) extends AnyVal
