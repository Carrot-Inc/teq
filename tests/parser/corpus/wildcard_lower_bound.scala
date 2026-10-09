// An explicit type argument whose lower bound is the receiver's wildcard element type, as
// izumi-reflect sorts a `Set[? <: T]` through `toArray[AbstractReference]`.
trait Ref
final case class Name(n: String) extends Ref

object Main:
  def sorted[T <: Ref](set: Set[? <: T]): List[Ref] =
    val array: Array[Ref] = set.toArray[Ref]
    array.toList
  def main(args: Array[String]): Unit =
    println(sorted(Set(Name("a"))))
