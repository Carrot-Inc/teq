// A wildcard argument captured where a value is adapted to the type expected of it (`Typer.adapt`'s
// `captureWildcardsCompat`, `Inferencing.captureWildcards`) is bounded by its parameter's bounds over the value's
// own arguments: `B <: F[Int]` of a `C[List, ?]` is a `List[Int]`, which is a `Seq[Int]`. The comparer's bounds of
// `B` against `C[?, ? <: Seq[Int]]` alone (`isSubArgs`' `paramBounds`, `F` the expected type's wildcard) are `Any`.
class C[F[_], B <: F[Int]]
def h(c: C[List, ?]): C[?, ? <: Seq[Int]] = c
def k(c: C[List, ?]): C[List, ? <: Seq[Int]] = c

@main def run(): Unit =
  val c = C[List, List[Int]]()
  println(h(c) eq c)
  println(k(c) eq c)
