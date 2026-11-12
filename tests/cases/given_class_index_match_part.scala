// `given_import_index_match_part` read lexically, in the body of
// the object that fixes the parameter: `Marker & Pick[T]`, an `A`
// through `Lib`, beats the outer `fallback` (scalac 3.8.4: 13).
trait Marker
class A(val n: Int)
type Pick[T] = T match
  case String => A
  case Int => Int
trait Library[T]:
  given picked: (Marker & Pick[T]) = (new A(13) with Marker {}).asInstanceOf[Marker & Pick[T]]
given fallback: A = A(99)
object Lib extends Library[String]:
  def get: Int = summon[A].n
object Main:
  def main(args: Array[String]): Unit =
    println(Lib.get)
