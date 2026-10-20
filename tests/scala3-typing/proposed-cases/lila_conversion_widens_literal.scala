// An implicit conversion on a receiver of literal type ("a".some, "a" -> v, cats' OptionIdOps
// and Predef's ArrowAssoc) or of a declared union of literals (ta.some, ta -> v) must keep that
// type when the expected type needs it.
import scala.language.implicitConversions

type T = "a" | "b"

final class OptionIdOps[A](private val a: A) extends AnyVal:
  def some: Option[A] = Some(a)
implicit def catsSyntaxOptionId[A](a: A): OptionIdOps[A] = new OptionIdOps(a)

@main def run() =
  val o1: Option[T] = "a".some
  val ta: T = "b"
  val o2: Option[T] = ta.some
  val m1: Map[T, Int] = Map("a" -> 1)
  val m2: Map[T, Int] = List(ta -> 2).map((k, v) => k -> v).toMap
  println((o1, o2, m1, m2))
