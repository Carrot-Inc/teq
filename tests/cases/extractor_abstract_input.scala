// An extractor whose input is an abstract type, over a scrutinee of a class (`Any`): tested at the
// parameter's fresh type (at its upper bound where the member is covariant in it, `Co[Any]`), a
// test that passes any value (scalac 3.8.4 warns that it is unchecked), the extractor applied to
// what the scrutinee is; the warnings are silenced (`@nowarn`).
import scala.annotation.nowarn
trait Types:
  type Type[A]
  def size[A](a: Type[A]): Int
  object Ex:
    def unapply[A](a: Type[A]): Some[Int] = Some(size(a))
  @nowarn def any(x: Any): Int = x match
    case Ex(n) => n
    case _ => -1
trait Plain:
  type T
  def show(t: T): String
  object Ex2:
    def unapply(a: T): Some[String] = Some(show(a))
  @nowarn def any(x: Any): String = x match
    case Ex2(s) => s
    case _ => "none"
trait Variant:
  type Co[+A]
  class Box[A](val a: A)
  def box[A](c: Co[A]): Box[A]
  object ExCo:
    def unapply[A](x: Co[A]): Some[Box[A]] = Some(box(x))
  @nowarn def any(x: Any): Box[Any] = x match
    case ExCo(v) => v
    case _ => new Box[Any]("none")
object ListVariant extends Variant:
  type Co[+A] = List[A]
  def box[A](c: List[A]): Box[A] = new Box(c.head)
object Lists extends Types:
  type Type[A] = List[A]
  def size[A](a: List[A]): Int = a.length
object Ints extends Plain:
  type T = Int
  def show(t: Int): String = s"int $t"

@main def run(): Unit =
  println(Lists.any(List(1, 2, 3)))
  println(Ints.any(4))
  println(ListVariant.any(List(7)).a)
