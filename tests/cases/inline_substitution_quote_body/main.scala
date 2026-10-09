// A quote and a quote pattern in an inline method's body, expanded where a macro's implementation
// calls it: their type parameters take the `Type` values the call passes (`twice(e)` with the
// macro's `T`), the holes and splices the expansion's arguments. scalac prints the lines of the
// .expected file.
@main def run(): Unit =
  val k = 3
  println(Q.pair(k + 1))
  println(Q.pair(true))
  println(Q.value)
  println(Q.d(k + 1))
  println(Q.d(true))
