// cats' newtype encoding (`NonEmptySet`): an abstract type bounded by an abstract type and a trait
// erases as scalac's `erasedGlb` orders the parts, a class before a trait, so `Base & Tag` is
// `Object` and a value of the underlying type passes; so do the other shapes of the order.
trait Newtype:
  type Base
  trait Tag extends Any
  type Type[A] <: Base & Tag

object Wrapped extends Newtype:
  def apply[A](s: Set[A]): Type[A] = s.asInstanceOf[Type[A]]
  def unwrap[A](t: Type[A]): Set[A] = t.asInstanceOf[Set[A]]

type Wrapped[A] = Wrapped.Type[A]

trait Marker
class Base
class Sub extends Base

object Main:
  val held: Wrapped[Int] = Wrapped(Set(1, 2))

  def tagged(x: Any): AnyRef & Marker = x.asInstanceOf[AnyRef & Marker]
  def narrowest(x: Base & Sub): Base & Sub = x

  def main(args: Array[String]): Unit =
    println(Wrapped.unwrap(held))
    println(Wrapped.unwrap(Wrapped(Set("a"))).size)
    println(tagged("not a marker") != null)
    println(narrowest(new Sub).getClass.getName)
