// A given whose type constructor parameter is bounded (`C[X] <: Set[X]`) is no candidate for a
// `List`, and the tighter bound is the more specific given for a `Set`, as under scalac.
trait Schema[T]:
  def show: String
object Schema:
  given Schema[Int] with
    def show = "int"
  implicit def forIterable[T: Schema, C[X] <: Iterable[X]]: Schema[C[T]] = new Schema[C[T]]:
    def show = "iterable(" + summon[Schema[T]].show + ")"
  implicit def forSet[T: Schema, C[X] <: Set[X]]: Schema[C[T]] = new Schema[C[T]]:
    def show = "set(" + summon[Schema[T]].show + ")"
  implicit def forMap[V: Schema]: Schema[Map[String, V]] = new Schema[Map[String, V]]:
    def show = "map(" + summon[Schema[V]].show + ")"

object Main:
  def main(args: Array[String]): Unit =
    println(summon[Schema[List[Int]]].show)
    println(summon[Schema[Set[Int]]].show)
    println(summon[Schema[Vector[Int]]].show)
    println(summon[Schema[Map[String, Int]]].show)
