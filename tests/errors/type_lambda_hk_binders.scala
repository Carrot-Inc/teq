// expect: 7:21: error: type X not found
// expect: 9:10: error: type argument Int does not have the same kind as its bound [X]
// expect: 2 errors found

// The clause of a type lambda's higher-kinded binder scopes over its bound alone, and the binder
// takes a type constructor of its arity.
type T = [F[X]] =>> X
type V = [F[X]] =>> F[Int]
val v: V[Int] = 1
