// teq: --source 3.8 --werror --wunused all
// A generator's binder is a pattern variable whatever its lambda (the review of 1888ac4df): `makeLambda`'s
// `IdPattern` parameter is `derivedValDef`'s, marked `PatternVar`.
@main def m =
  for x <- List(1) do println("a")
  for y: Int <- List(1) do println("b")
  for (z) <- List(1) do println("c")
  for (p, q) <- List((1, 2)) do println(p)
  for X <- List(1) do println("d")
// expect: 5:7: warning: unused pattern variable
// expect: 6:7: warning: unused pattern variable
// expect: 7:8: warning: unused pattern variable
// expect: 8:11: warning: unused pattern variable
// expect: 9:7: warning: unused pattern variable
