// `Tuple`'s companion as scala-library declares it: `Tuple(x)` a one-element tuple, `Tuple.unapply` of the empty
// tuple, and the `CanEqual` givens of empty and non-empty tuples a strict-equality program finds.
import scala.language.strictEquality
@main def run(): Unit =
  println(Tuple(1))
  println(Tuple() == EmptyTuple)
  println(Tuple.unapply(EmptyTuple))
  println(summon[CanEqual[EmptyTuple, EmptyTuple]] != null)
  println(Tuple.canEqualEmptyTuple != null)
  println((1, "a") == (1, "a"))
  println(EmptyTuple == EmptyTuple)
