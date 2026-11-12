// An alias application whose argument the kind check refuses, inside
// a match type's scrutinee: dotty's `errorTree` makes the argument
// the error type, the expected type `M[L[<error>]]` is erroneous and
// dotty's typer reports no mismatch against it: scalac 3.8.4 reports the
// kind error alone (1 error). The .expected holds scalac's one error.
type L[F[_]] = F[Int]
type M[T] = T match
  case Option[x] => x
  case _ => String
val a: M[L[Option[Double]]] = "s"
