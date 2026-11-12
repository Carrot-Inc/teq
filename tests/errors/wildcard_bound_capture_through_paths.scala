trait BS[T, S <: BS[T, S]]
class Impl[T] extends BS[T, Impl[T]]
class C[A <: C[A]]
class D extends C[D]
trait H { type S <: BS[?, S] }
trait H1 { type S <: BS[?, S] }
trait H2 { type S <: AnyRef }
trait M[A <: M[A, B], B <: M[A, B]]
trait IM extends M[IM, IM]
object Shapes:
  def a01(x: BS[?, Impl[?]]): Int = 1
  def a03(x: C[? >: D <: C[?]]): Int = 1
  def a08(h: H)(x: BS[?, h.S]): Int = 1
  def a09(h: H1 & H2)(x: BS[?, h.S]): Int = 1
  def a10[A <: M[A, B], B <: M[A, B]](x: M[?, B]): Int = 1
  type B07 = M[?, IM]
trait Sub extends H { def c01: BS[?, S] }
object Obj extends H { def c02: BS[?, S] = ??? }
trait Low { type S <: BS[?, S]; type R <: S; def c06: BS[?, R] }
trait Both extends H1 with H2 { def c07: BS[?, S] }
trait Own { type S <: BS[?, S]; def ok: BS[?, S] }
// `boundsViolations`' check of an F-bounded parameter's argument where another argument is a wildcard (TypeOps.scala
// 630-701): the skolemized `<app>#T` in the bound, `compareCaptured`'s capture of the left root's wildcard
// (TypeComparer.scala 1870-1891) and `narrowBound`'s GADT state, each beside scalac 3.8.4's E057.
// A point argument that is an application with a wildcard (`Impl[?]`), a
// lower-bounded wildcard's overlap, an abstract member's reference through a path, through an intersection prefix,
// seen from a subtrait, an object and an intersection's trait, a member bounded by the F-bounded one, and mutual
// F-bounds over two parameters are rejected; the member's own `BS[?, S]` inside its trait (`Own.ok`) is accepted
// (the GADT bound `narrowBound` installs is in `H.this` terms, which the path `h.S` and the subtrait's `Sub.this.S`
// are not). scalac's columns are 0-based, the selection's point its name (`h.S`'s `S`).
// expect: wildcard_bound_capture_through_paths.scala:11:20: error: type argument Impl[?] does not conform to upper bound BS[BS[?, Impl[?]]#T, Impl[?]]
// expect: wildcard_bound_capture_through_paths.scala:12:16: error: type argument ? >: D <: C[?] does not overlap with upper bound C[C[? >: D <: C[?]]#A]
// expect: wildcard_bound_capture_through_paths.scala:13:26: error: type argument h.S does not conform to upper bound BS[BS[?, h.S]#T, h.S]
// expect: wildcard_bound_capture_through_paths.scala:14:32: error: type argument h.S does not conform to upper bound BS[BS[?, h.S]#T, h.S]
// expect: wildcard_bound_capture_through_paths.scala:15:47: error: type argument B does not conform to upper bound M[M[?, B]#A, B]
// expect: wildcard_bound_capture_through_paths.scala:16:19: error: type argument IM does not conform to upper bound M[M[?, IM]#A, IM]
// expect: wildcard_bound_capture_through_paths.scala:17:38: error: type argument Sub.this.S does not conform to upper bound BS[BS[?, Sub.this.S]#T, Sub.this.S]
// expect: wildcard_bound_capture_through_paths.scala:18:39: error: type argument Obj.S does not conform to upper bound BS[BS[?, Obj.S]#T, Obj.S]
// expect: wildcard_bound_capture_through_paths.scala:19:61: error: type argument Low.this.R does not conform to upper bound BS[BS[?, Low.this.R]#T, Low.this.R]
// expect: wildcard_bound_capture_through_paths.scala:20:48: error: type argument Both.this.S does not conform to upper bound BS[BS[?, Both.this.S]#T, Both.this.S]
// expect: 10 errors found
