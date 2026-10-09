trait Show[T]
trait Eq[T]
trait U
trait L extends U
trait Convert[A, B]
type Conv[A] = [B] =>> Convert[A, B]
object Bounds:
  def single[T: Show](x: T): T = x
  def multi[T: Show: Eq](x: T): T = x
  class Box[T: Show](val x: T)
  extension [T: Show](x: T)
    def shown: T = x
  def applied[T: Conv[Int]](x: T): T = x
  def upper[T <: U](x: T): T = x
  def lower[T >: L](x: T): T = x
  def explicit[T](x: T)(using s: Show[T]): T = x
  given made: Show[Int] = new Show[Int] {}
