// teq: --source 3.3 --werror
// `(K)` is no `IdPattern` (the review of 1888ac4df): under `CheckAndFilter` the generator's case lambda is checked,
// its stable identifier pattern refutable against its `x$1`, a warning before 3.4; `K` is the function's parameter.
val K = 1
@main def m =
  for (K) <- List(1, 2) do println("k")
  for K <- List(1, 2) do println(K)
// expect: 6:8: warning: pattern's type (K : Int) does not match the right hand side expression's type (x$1 : Int)
// absent: for_paren_stable_id_3_3.scala:7:
