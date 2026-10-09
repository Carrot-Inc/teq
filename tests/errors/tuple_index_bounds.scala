// A constant index past a tuple type's elements is scalac's error, where `Tuple.Elem` does not
// reduce: a literal, an operation over constants, a `final val` (a nested object's too), a val of
// a literal type, an inline `def` of one or a transparent one of a constant, a named argument; in
// an inline body, where its expansion reaches it, at the expansion's call: a constant there, an
// inline or a plain parameter given one, an operation over one, a `constValue`, an index into a
// tuple a type parameter stood for; an operation over an ascription, which scalac folds, and a
// `final val` of an operation over constants. The runtime `apply` is for an index the type does
// not give.
// expect: 47:18: error: index out of bounds: 2
// expect: 48:18: error: index out of bounds: -1
// expect: 49:95: error: index out of bounds: 23
// expect: 50:18: error: index out of bounds: 5
// expect: 51:27: error: index out of bounds: 3
// expect: 52:18: error: index out of bounds: 2
// expect: 53:11: error: index out of bounds: 2
// expect: 55:18: error: index out of bounds: 2
// expect: 56:18: error: index out of bounds: 2
// expect: 57:18: error: index out of bounds: 2
// expect: 58:18: error: index out of bounds: 2
// expect: 59:11: error: index out of bounds: 2
// expect: 60:11: error: index out of bounds: 2
// expect: 61:11: error: index out of bounds: 2
// expect: 62:11: error: index out of bounds: 2
// expect: 63:11: error: index out of bounds: 2
// expect: 64:11: error: index out of bounds: 2
// expect: 65:18: error: index out of bounds: 2
// expect: 66:18: error: index out of bounds: 2
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
  println((1, 2)(2))
  println((1, 2)(-1))
  println((1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)(23))
  println((1, 2)(K.k))
  println((1, 2, 3).apply(n = 3))
  println((1, 2)(1 + 1))
  println(reaches(true))
  val two: 2 = 2
  println((1, 2)(two))
  println((1, 2)(K.Nested.n))
  println((1, 2)(lit))
  println((1, 2)(folded))
  println(pick(2))
  println(plain(2))
  println(next(1))
  println(nth[2])
  println(generic((1, 2)))
  println(local)
  println((1, 2)((2: Int) + 0))
  println((1, 2)(K.sum))
