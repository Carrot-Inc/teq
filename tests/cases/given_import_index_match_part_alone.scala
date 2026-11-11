// `given_import_index_match_part` with no other candidate: the imported `Marker & Pick[T]` is the `A` the search
// takes through `Lib` (scalac 3.8.4: 13).
trait Marker
class A extends Marker:
  def value = 13
trait Library[T]:
  type Pick[X] = X match
    case String => A
    case _ => X
  given mixed: (Marker & Pick[T]) = new A().asInstanceOf[Marker & Pick[T]]
object Lib extends Library[String]
object Main:
  def main(args: Array[String]): Unit =
    import Lib.mixed
    println(summon[A].value)
