// expect: 8:12: error: type argument Option does not conform to upper bound [X] =>> Iterable[X]
// expect: 1 error found

// A type lambda's higher-kinded binder holds its argument to its bound, as scalac's
// `Checking.checkAppliedType` does (E057).
type W = [F[X] <: Iterable[X]] =>> F[Int]
@main def run(): Unit =
  val w: W[Option] = Some(7)
  println(w)
