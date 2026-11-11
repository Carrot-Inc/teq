// A part that is an alias of a bounded type parameter (`Id[T]`, `type Id[X] = X`, `T <: Base`) is the type the
// prefix gives (`Sub` through `Lib extends Library[Sub]`): scalac 3.8.4 prints 13. teq's class test reads the part
// through `class_of`, which dealiases `Id[T]` and follows `T`'s bound to `Base` (`inter_classes_known`,
// `may_provide`), and rejects the given for `Sub`, so the outer `fallback` wins: 99, on master too.
trait Marker
class Base(val n: Int)
class Sub(n: Int) extends Base(n)
type Id[X] = X
trait Library[T <: Base]:
  def make: T
  given picked: (Marker & Id[T]) = (new Sub(13) with Marker {}).asInstanceOf[Marker & Id[T]]
object Lib extends Library[Sub]:
  def make = Sub(1)
given fallback: Sub = Sub(99)
object Main:
  def main(args: Array[String]): Unit =
    import Lib.given
    println(summon[Sub].n)
