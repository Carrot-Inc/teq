// The shapes refined's `Validate` instances take: a given whose type is a refinement
// (`Validate.Aux[T, P, R]`) found through the implicit scope of the refined target, a path's
// abstract type member equal to itself in a refinement (`v.R`), a dependent result type
// (`Not[v.Res]`), an evidence `T => Iterable[?]` answered by `wrapString` for a `String`,
// `Ordering.max[U <: T]`, and `G[A](f)` for a nullary `apply[A]` whose result overloads `apply`.
trait Result[A]:
  def show: String
case class Passed[A](a: A) extends Result[A]:
  def show = "passed(" + a + ")"
case class Failed[A](a: A) extends Result[A]:
  def show = "failed(" + a + ")"

trait Validate[T, P]:
  type R
  final type Res = Result[R]
  def validate(t: T): Res

object Validate:
  type Aux[T, P, R0] = Validate[T, P] { type R = R0 }
  type Plain[T, P] = Aux[T, P, P]
  def apply[T, P](implicit v: Validate[T, P]): Aux[T, P, v.R] = v
  def fromPredicate[T, P](f: T => Boolean, p: P): Plain[T, P] =
    new Validate[T, P]:
      type R = P
      def validate(t: T): Res = if f(t) then Passed(p) else Failed(p)

object boolean:
  final case class Not[P](p: P)
  object Not:
    implicit def notValidate[T, P, R](implicit v: Validate.Aux[T, P, R]): Validate.Aux[T, Not[P], Not[v.Res]] =
      new Validate[T, Not[P]]:
        type R = Not[v.Res]
        def validate(t: T): Res = v.validate(t) match
          case Passed(r) => Failed(Not(Passed(r)))
          case Failed(r) => Passed(Not(Failed(r)))

object numeric:
  final case class Greater[N](n: N)
  object Greater:
    implicit def greaterValidate[T, N](implicit nt: Numeric[T]): Validate.Plain[T, Greater[N]] =
      Validate.fromPredicate[T, Greater[N]](t => nt.gt(t, nt.zero), Greater(null.asInstanceOf[N]))
  type Positive = Greater[0]
  type NonPositive = boolean.Not[Positive]

object collection:
  final case class Empty()
  object Empty:
    implicit def emptyValidate[T](implicit ev: T => Iterable[?]): Validate.Plain[T, Empty] =
      Validate.fromPredicate[T, Empty](t => ev(t).isEmpty, Empty())
  type NonEmpty = boolean.Not[Empty]

class Mk[A]:
  def apply(): Int = 0
  def apply[B](f: A => B): String = "mk"
object Mk:
  def apply[A]: Mk[A] = new Mk[A]

object Main:
  def sizeOf[T](t: T)(implicit ev: T => Iterable[?]): Int = ev(t).size
  def main(args: Array[String]): Unit =
    println(Validate[Int, numeric.Positive].validate(3).show)
    println(Validate[Int, numeric.NonPositive].validate(3).show)
    println(Validate[Int, numeric.NonPositive].validate(-3).show)
    println(Validate[String, collection.Empty].validate("").show)
    println(Validate[String, collection.NonEmpty].validate("ab").show)
    println(Validate[List[Int], collection.NonEmpty].validate(Nil).show)
    println(sizeOf("abc") + sizeOf(List(1, 2)))
    val chars: Iterable[Char] = "xy"
    println(chars.toList)
    println(Ordering[Int].max(3, 5) + " " + Ordering[Int].min(3, 5) + " " + Numeric[Int].max(3, 5))
    println(Mk[Int](_ + 1) + " " + Mk[Int]())
