// expect: 12:21: error: type mismatch: found String, required Box[Int]
// A given conversion whose using clause fixes its result to another type than the required one is
// no conversion there (scalac: E007 Found ("input" : String), Required: Box[Int]).
import scala.language.implicitConversions
final case class Box[A](value: A)
trait Default[A]:
  def value: A
given Default[String] with
  def value = "d"
given box[X, A](using d: Default[A]): Conversion[X, Box[A]] = _ => Box(d.value)
@main def main(): Unit =
  val b: Box[Int] = "input"
  println(b.value + 1)
