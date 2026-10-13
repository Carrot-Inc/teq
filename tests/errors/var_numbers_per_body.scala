// expect: 8:31: error: type mismatch: found (Int), required F[A, B]
// expect: 9:31: error: type mismatch: found (Int), required F[A, B]
// expect: 2 errors found
// An open higher-kinded variable a message shows is named by its type parameter, as dotty shows
// `F[A, B]` (its where-clause aside): each body's message is the same, whatever the build
// typed before it.
def arity[F[_, _], A, B](x: F[A, B]): String = "two"
def a(): Unit = println(arity(Tuple1(1)))
def b(): Unit = println(arity(Tuple1(2)))
