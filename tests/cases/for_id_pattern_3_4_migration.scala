// teq: --source 3.4-migration --wconf any:s
// `(K)` is no `IdPattern`: a case lambda checked under `Check`, its refutable pattern a
// migration warning, unfiltered.
val K = 1
@main def Test =
  for (K) <- List(1, 1) do println("k")
