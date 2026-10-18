// teq: --source 3.4-migration --werror
// `(K)` is no `IdPattern` (the review of 1888ac4df): checked under `Check`, a migration warning at 3.4-migration.
val K = 1
@main def m =
  for (K) <- List(1, 1) do println("k")
// expect: 5:8: migration warning: pattern's type (K : Int) does not match the right hand side expression's type (x$1 : Int)
