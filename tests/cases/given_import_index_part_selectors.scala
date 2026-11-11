// `given_import_index_match_part_alone` through a given wildcard and through a renaming selector: both bring the
// given whose part has no class (scalac 3.8.4: 13, 13).
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
    locally { import Lib.given; println(summon[A].value) }
    locally { import Lib.mixed as renamed; println(summon[A].value) }
