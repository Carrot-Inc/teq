// A wildcard argument of a parent is refused whatever its bounds: a parent is typed as a constructor call, and
// `typedNew` reports each `TypeBoundsTree` argument (Typer.scala 1273-1276, `WildcardOnTypeArgumentNotAllowedOnNew`).
// scalac 3.8.4 rejects both parents (E084, a syntax error):
//   10:19 Type argument must be fully defined
//   11:19 Type argument must be fully defined
// teq is to reject both (not a bound check; with an out-of-bounds wildcard,
// `extends C[? >: String]`, scalac gives this E084 too).
class C[T <: Int]
class B[X]
class E1 extends C[? <: Int]
class E2 extends B[?]
