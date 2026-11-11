// `given_import_index_match_part` through the implicit scope: `A`'s companion extends `Library[String]`, and its
// `Marker & Pick[T]` is an `A` (scalac 3.8.4: 13).
trait Marker
class A(val n: Int)
type Pick[T] = T match
  case String => A
  case Int => Int
trait Library[T]:
  given picked: (Marker & Pick[T]) = (new A(13) with Marker {}).asInstanceOf[Marker & Pick[T]]
object A extends Library[String]
object Main:
  def main(args: Array[String]): Unit =
    println(summon[A].n)
