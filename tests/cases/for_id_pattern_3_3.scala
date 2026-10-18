// teq: --source 3.3 --wconf any:s
// `(K)` is no `IdPattern` (the review of 1888ac4df): under `CheckAndFilter` the generator filters the stable
// identifier pattern and warns of it; `K` is the function's parameter.
val K = 1
@main def Test =
  for (K) <- List(1, 2) do println("k")
  for K <- List(1, 2) do println(K)
