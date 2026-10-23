import scala.annotation.unchecked.{uncheckedVariance as U}
class Box[A]
object Unchecked {
  def source: Box[Int @U] = ???
  val result = source
  def sourceWithSet: Box[(Int & String) @U] = ???
  val resultWithSet = sourceWithSet
}
