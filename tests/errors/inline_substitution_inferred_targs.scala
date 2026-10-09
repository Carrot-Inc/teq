// expect: 10:25: error: type mismatch: found List[B], required List[O]
// expect: 1 error found
// The type arguments a transparent inline body infers at its definition are kept at the expansion:
// `List(x)` over `x: B` is a `List[B]` for `O` as well, as scalac 3.8.4 keeps it (`Found:
// List[B], Required: List[O.type]`); the retype path infers `List[O.type]` and accepts the program.
trait B
object O extends B
transparent inline def f(x: B) = List(x)
@main def run(): Unit =
  val r: List[O.type] = f(O)
  println(r)
