// A wildcard's bound check reads the other arguments of its application in the parameter's bound (scala3's
// tests/neg/tate.scala shape): `? >: String` for `B <: A` with `A := Int` needs `String <: Int`, `? <: T` for
// `B >: A` with `A := U` needs `U <: T`, and a type lambda's or a higher-kinded argument's own parameter `X` (bounded
// by `Any`) is no `Int`. scalac 3.8.4 (E057):
//   10:18 Type argument  >: String does not overlap with upper bound Int
//   13:24 Type argument  <: T does not overlap with lower bound U
//   17:19 Type argument  >: X does not overlap with upper bound Int
//   19:21 Type argument  >: X does not overlap with upper bound Int
class Up[A, B <: A]
type X1 = Up[Int, ? >: String]
trait Lo[A, B >: A]
def coerce[T, U](t: T): U =
  lazy val bound: Lo[U, ? <: T] = ???
  ???
class C[T <: Int]
class K[F[_]]
type L = [X] =>> C[? >: X]
type M = [X <: Int] =>> C[? >: X]
type N = K[[X] =>> C[? >: X]]
type Ok = Up[CharSequence, ? >: String]
// expect: 10:19: error: type argument ? >: String does not overlap with upper bound Int
// expect: 13:25: error: type argument ? <: T does not overlap with lower bound U
// expect: 17:20: error: type argument ? >: X does not overlap with upper bound Int
// expect: 19:22: error: type argument ? >: X does not overlap with upper bound Int
// expect: 4 errors found
