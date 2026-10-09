// teq: --std=scala-library
// expect: 7:17: error: type mismatch: found (Int), required ?0[A, B]
// `Tuple1[X]` has no base of two arguments: its base `X *: EmptyTuple` is itself.
def arity[F[_, _], A, B](x: F[A, B]): String = "two"

@main def run(): Unit =
  println(arity(Tuple1(1)))
  println(arity((1, "a")))
