// `given_import_index_match_part_alone` through an import from a value (`lib: Library[String]`), whose implicits are
// not kept: the index decides afresh and keeps the given (scalac 3.8.4: 19).
trait Marker
class A extends Marker:
  def value = 19
trait Library[T]:
  type Pick[X] = X match
    case String => A
    case _ => X
  given mixed: (Marker & Pick[T]) = new A().asInstanceOf[Marker & Pick[T]]
class L extends Library[String]
object Main:
  def use(lib: Library[String]): Int =
    import lib.mixed
    summon[A].value
  def main(args: Array[String]): Unit = println(use(new L))
