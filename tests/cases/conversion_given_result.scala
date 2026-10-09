// A given conversion's using clause is resolved for the result the conversion is required to give:
// `Default[Int]` for a `Box[Int]`, next to a `Default[String]` (scalac: "42").
import scala.language.implicitConversions
final case class Box[A](value: A)
trait Default[A]:
  def value: A
given Default[String] with
  def value = "d"
given Default[Int] with
  def value = 41
given box[X, A](using d: Default[A]): Conversion[X, Box[A]] = _ => Box(d.value)
@main def main(): Unit =
  val b: Box[Int] = "input"
  println(b.value + 1)
