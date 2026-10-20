// A constant index past a tuple type's elements is scalac's error, where `Tuple.Elem` does not
// reduce: a literal, an operation over constants, a `final val` (a nested object's too), a val of
// a literal type, an inline `def` of one or a transparent one of a constant, a named argument; an
// operation over an ascription, which scalac folds, and a `final val` of an operation over
// constants. scalac reports it in `TupleOptimizations`, after `Inlining`: teq's error is late,
// holding back no expansion (tuple_index_bounds_inlined has the indexes an expansion reaches).
// expect: 29:18: error: index out of bounds: 2
// expect: 30:18: error: index out of bounds: -1
// expect: 31:95: error: index out of bounds: 23
// expect: 32:18: error: index out of bounds: 5
// expect: 33:27: error: index out of bounds: 3
// expect: 34:18: error: index out of bounds: 2
// expect: 36:18: error: index out of bounds: 2
// expect: 37:18: error: index out of bounds: 2
// expect: 38:18: error: index out of bounds: 2
// expect: 39:18: error: index out of bounds: 2
// expect: 40:18: error: index out of bounds: 2
// expect: 41:18: error: index out of bounds: 2
// expect: 12 errors found
import scala.compiletime.constValue
object K:
  final val k = 5
  final val sum = 1 + 1
  object Nested:
    final val n = 2
inline def lit: 2 = 2
transparent inline def folded = 2
@main def run(): Unit =
  println((1, 2)(2))
  println((1, 2)(-1))
  println((1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)(23))
  println((1, 2)(K.k))
  println((1, 2, 3).apply(n = 3))
  println((1, 2)(1 + 1))
  val two: 2 = 2
  println((1, 2)(two))
  println((1, 2)(K.Nested.n))
  println((1, 2)(lit))
  println((1, 2)(folded))
  println((1, 2)((2: Int) + 0))
  println((1, 2)(K.sum))
