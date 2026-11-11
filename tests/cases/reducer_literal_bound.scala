// `ApproximatingTypeMap#derivedAppliedType`'s result guard (Types.scala 6771-6777): `B`'s bound `F[A]` over
// `F := [X] =>> 1` and a wildcard `A` reduces to the literal `1`, which is no unreducible application, so the
// unbounded `?` given for `B` conforms to `? <: 1`.
class ReducerLiteralC[F[_], A, B <: F[A]]

object ReducerLiteral:
  def h(c: ReducerLiteralC[[X] =>> 1, ?, ?]): ReducerLiteralC[[X] =>> 1, ?, ? <: 1] = c

@main def run(): Unit =
  val c = new ReducerLiteralC[[X] =>> 1, Int, 1]
  println(ReducerLiteral.h(c) eq c)
