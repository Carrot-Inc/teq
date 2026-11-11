// An alias application whose argument the kind check refuses, inside a match type's scrutinee: dotty's `errorTree`
// makes the argument the error type, the expected type `M[L[<error>]]` is erroneous and the typer reports no
// mismatch against it: scalac 3.8.4 reports the kind error alone (1 error). teq reports it and then "type mismatch:
// found "s", required M[L[<error>]]" (the round's base the same with `M[L[Option[Double]]]`). Both reject; the
// .expected holds scalac's one error. The closure adversary of the pass on 5bd85bcd1 (round 23).
type L[F[_]] = F[Int]
type M[T] = T match
  case Option[x] => x
  case _ => String
val a: M[L[Option[Double]]] = "s"
