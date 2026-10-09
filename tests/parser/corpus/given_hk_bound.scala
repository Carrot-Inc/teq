// A given over a higher-kinded parameter with a bound (`T[X] <: Iterable[X]`, zio-json's
// `iterable` encoder) applies only to constructors within the bound: a `Box[String]` is not one,
// so the search takes the codec's route instead of encoding the box as a collection.
trait Enc[A]:
  def name: String
trait Codec[A]:
  def name: String

trait LowPriority:
  given iterable[A, T[X] <: Iterable[X]](using e: Enc[A]): Enc[T[A]] with
    def name = "iterable(" + e.name + ")"

object Enc extends LowPriority:
  given string: Enc[String] with
    def name = "string"
  given fromCodec[A](using c: Codec[A]): Enc[A] with
    def name = "codec(" + c.name + ")"

final case class Box[A](a: A)
object Box:
  given [A](using c: Codec[A]): Codec[Box[A]] with
    def name = "box(" + c.name + ")"
given Codec[String] with
  def name = "str"

def pick[T[X] <: Iterable[X], A](t: T[A]): String = "ok"

object Main:
  def main(args: Array[String]): Unit =
    println(summon[Enc[Box[String]]].name)
    println(summon[Enc[List[String]]].name)
    println(summon[Enc[Vector[Box[String]]]].name)
    println(pick(List(1)))
