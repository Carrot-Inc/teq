// A given of an intersection with a part of no class of its own (`Pick[T]`, a match type `T` leaves stuck) beside one
// of a class (`Marker`) is the type the import's prefix gives: `Pick[String]` is `A` through `Lib`, as dotty's
// `filterMatching` reads the reference (`NoViewsAllowed.isCompatible`, Implicits.scala:239-246). The import's index
// keeps it for every class, as the class test passes it, so it beats the outer `fallback`: scalac 3.8.4 prints 13.
trait Marker
class A(val value: Int) extends Marker
trait Library[T]:
  type Pick[X] = X match
    case String => A
    case _ => X
  given mixed: (Marker & Pick[T]) = new A(13).asInstanceOf[Marker & Pick[T]]
object Lib extends Library[String]
object Fallback:
  given fallback: A = new A(99)
import Fallback.fallback
object Main:
  def main(args: Array[String]): Unit =
    import Lib.mixed
    println(summon[A].value)
