// expect: 17:30: error: type argument ? >: T does not overlap with upper bound U
// expect: 1 error found
// A bounded wildcard argument is checked against its parameter's bounds by its ends (`TypeOps.boundsViolations`'
// `checkOverlapsBounds`, TypeOps.scala 683-702, 732-733): `_ >: T` for `B <: A` with `A := U` needs `T <: U`.
// scalac 3.8.4 reports it at the argument:
//   -- [E057] Type Mismatch Error: tate.scala:7:33
//   Type argument  >: T does not overlap with upper bound U
// and nothing else: the call below, the wildcard captured with its parameter's bounds (`Inferencing.
// captureWildcards`), typechecks. Without the check the program is accepted (scala3's tests/neg/tate.scala, the
// gate of 861d8acf1).
object unsound {
  trait Bound[A, B <: A]
  trait Bind[A] {
    def bad[B <: A](bound: Bound[A, B], b: B) = b
  }
  def coerce[T, U](t: T): U = {
    lazy val bound: Bound[U, ? >: T] = ???
    def bind = new Bind[U] {}
    bind.bad(bound, t)
  }
}
