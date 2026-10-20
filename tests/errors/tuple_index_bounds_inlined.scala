// A constant index past a tuple type's elements in an inline body, where its expansion reaches it,
// at the expansion's call: a constant there, an inline or a plain parameter given one, an operation
// over one, a `constValue`, an index into a tuple a type parameter stood for (scalac's
// `TupleOptimizations`, after `Inlining`).
// expect: 32:11: error: index out of bounds: 2
// expect: 33:11: error: index out of bounds: 2
// expect: 34:11: error: index out of bounds: 2
// expect: 35:11: error: index out of bounds: 2
// expect: 36:11: error: index out of bounds: 2
// expect: 37:11: error: index out of bounds: 2
// expect: 38:11: error: index out of bounds: 2
// expect: 7 errors found
import scala.compiletime.constValue
object K:
  final val k = 5
  final val sum = 1 + 1
  object Nested:
    final val n = 2
inline def reaches(inline b: Boolean): Any =
  inline if b then (1, 2)(2) else 42
inline def lit: 2 = 2
transparent inline def folded = 2
inline def pick(inline i: Int): Any = (1, 2)(i)
inline def plain(i: Int): Any = (1, 2)(i)
inline def next(inline i: Int): Any = (1, 2)(i + 1)
inline def nth[N <: Int]: Any = (1, 2)(constValue[N])
inline def generic[T <: Tuple](t: T): Any = t(2)
inline def local: Any =
  val i: 2 = 2
  (1, 2)(i)
@main def run(): Unit =
  println(reaches(true))
  println(pick(2))
  println(plain(2))
  println(next(1))
  println(nth[2])
  println(generic((1, 2)))
  println(local)
