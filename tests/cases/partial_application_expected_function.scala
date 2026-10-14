// A partial application of a curried method against a function type expected (Airstream's
// `updateParent = standardErrorsF { (_, next) => .. }`): the clauses left out, which eta-expansion makes a
// function of, are related to the expected type before the written arguments are typed, as dotty's
// `constrainResult` compares a method type `normalize`d to a function type, so that the lambda's parameters take
// the types the expected function fixes (`P`, `T`).
import scala.util.{Try, Success, Failure}

object Vars:
  def standardErrorsF[P, T](update: (P, T) => Option[P])(parentTry: Try[P], nextTry: Try[T]): Option[Try[P]] =
    (parentTry, nextTry) match
      case (Success(p), Success(t)) => update(p, t).map(Success(_))
      case (Failure(e), _) => Some(Failure(e))
      case (_, Failure(e)) => Some(Failure(e))

  def curried3[A, B, C](f: (A, B) => C)(a: A)(b: B): C = f(a, b)

class Derived[A](val updateParent: (Try[Option[A]], Try[A]) => Option[Try[Option[A]]])

@main def main(): Unit =
  val d = new Derived[Int](Vars.standardErrorsF { (_, next) => Some(Some(next)) })
  println(d.updateParent(Success(None), Success(3)))
  val g: (Try[Int], Try[String]) => Option[Try[Int]] = Vars.standardErrorsF((p, s) => Some(p + s.length))
  println(g(Success(1), Success("ab")))
  val h: Int => String => String = Vars.curried3((n, s) => s * n)
  println(h(3)("ab"))
