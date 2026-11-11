// `TypeApplications.appliedTo`'s `Reducer` (TypeApplications.scala 113-156): `B`'s bound `F[A]` over
// `F := [X] =>> X` and the wildcard `A := ? <: String` reduces to `String` (the wildcard's upper bound), so the
// unbounded `?` given for `B` conforms to `? <: String`.
class ReducerC[F[_], A, B <: F[A]]

object Reducer:
  def h(c: ReducerC[[X] =>> X, ? <: String, ?]): ReducerC[[X] =>> X, ? <: String, ? <: String] = c

@main def run(): Unit =
  val c = new ReducerC[[X] =>> X, String, String]
  println(Reducer.h(c) eq c)
