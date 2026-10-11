// A named tuple's `size` on teq's own standard library, as scala-library's
// `NamedTupleDecomposition.size` has it: the size as a literal type of the tuple's (`Tuple.Size`).
@main def run() =
  val x = (name = "a", age = 1)
  println(x.toTuple)
  println(x.size)
  val s: 2 = x.size
  println(s)
  val y = (a = 1, b = 'b', c = "c")
  println(y.size + x.size)
