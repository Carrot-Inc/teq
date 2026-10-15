// jars: regroup-b regroup-a2
// A library compiled against version 1 of its dependency, run with version 2 (munit-cats-effect over munit
// 1.2): its call `check(a, b, clue)(loc, compare)` names by its signature the method version 2 keeps for
// binary compatibility, which takes the five arguments in one clause; erasure flattens the clauses alike,
// so the body's arguments are taken in the member's clauses.
@main def run(): Unit =
  println(rgb.Client.run(3, 3))
  println(rgb.Client.run(3, 4))
