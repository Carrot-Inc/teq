// expect: 8:31: error: type mismatch: found (Int), required ?0[A, B]
// expect: 9:31: error: type mismatch: found (Int), required ?0[A, B]
// expect: 2 errors found
// An inference variable a message shows unnamed is numbered by its place among the variables of
// the typing that made it: each body's first is `?0`, whatever the build
// typed before it.
def arity[F[_, _], A, B](x: F[A, B]): String = "two"
def a(): Unit = println(arity(Tuple1(1)))
def b(): Unit = println(arity(Tuple1(2)))
