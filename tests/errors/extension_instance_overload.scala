// expect: 9:33: error: None of the overloaded alternatives of method pair in trait Ops with types
// expect: 1 error found
// Same-named extensions of an instance resolve on the arguments typed alone (`resolveOverloaded`, `pretypeArgs`): the
// tuple `(1, 2)` is an `(Int, Int)`, which the `(Double, Double)` alternative does not take, and `true` no `Int`.
trait Ops:
  extension [T](x: Int) def pair(y: (Double, Double), z: Boolean): String = "double"
  extension [T](x: Int) def pair(y: (Int, Int), z: Int): String = "int"
object O extends Ops
@main def run(): Unit = println(O.pair[Unit](1)((1, 2), true))
