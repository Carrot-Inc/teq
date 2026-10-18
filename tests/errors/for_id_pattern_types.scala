// teq: --source 3.8
// A typed `IdPattern` is the function's parameter at its written type, checked where the closure is adapted to the
// expected function type (the review of 1888ac4df): `typedClosure`'s mismatch at the closure, pattern to body,
// under one parameter and under the N parameters `patternsToParams` makes.
@main def m =
  for x: String <- List(1) do println(x)
  val xs = List(1, 2); val ys = List(3, 4)
  for (a: String, b) <- xs.lazyZip(ys) do println(b)
// expect: 6:7: error: type mismatch: found String => Unit, required Int => Any
// expect: 8:7: error: type mismatch: found (String, Int) => Unit, required (Int, Int) => Any
