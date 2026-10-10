// expect: 6:14: error: type mismatch: found Int | String, required Int
// expect: 1 error found
// A plain inline method's call whose result is inferred has the definition's inferred type, not its
// expansion's (`Namer.inferredResultType`): `pick(true)` expands to `1` and is an `Int | String`.
inline def pick(inline b: Boolean) = if b then 1 else "s"
val n: Int = pick(true)
