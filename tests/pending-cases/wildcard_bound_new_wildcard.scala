// A wildcard argument of an instance creation is refused (Typer.scala 1273-1276, `typedNew`'s
// `WildcardOnTypeArgumentNotAllowedOnNew`). scalac 3.8.4 rejects it (E084, a syntax error):
//   6:14 Type argument must be fully defined
// teq accepts it (not a bound check: the wildcard is within `T <: Int`).
class C[T <: Int]
def f = new C[? <: Int]
