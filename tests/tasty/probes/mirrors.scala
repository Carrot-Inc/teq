// The synthesized mirrors: a product, a sum of a sealed trait
// without a companion mirror, an enum's, a case object's, a tuple's, and one a context bound takes.
package probe.mirrors
import scala.deriving.Mirror
case class P(a: Int, b: String)
enum Color:
  case Red, Green
sealed trait Shape
case class Circle(r: Int) extends Shape
case object Dot extends Shape
object Use:
  def m: Mirror.ProductOf[P] = summon[Mirror.ProductOf[P]]
  def s: Mirror.SumOf[Shape] = summon[Mirror.SumOf[Shape]]
  def c: Mirror.Of[Color] = summon[Mirror.Of[Color]]
  def d: Mirror.ProductOf[Dot.type] = summon[Mirror.ProductOf[Dot.type]]
  def t: Mirror.ProductOf[(Int, String)] = summon[Mirror.ProductOf[(Int, String)]]
  def gen[A](using m: Mirror.ProductOf[A]): Int = 1
  def g: Int = gen[P]
