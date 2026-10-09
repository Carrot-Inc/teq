// expect: 7:17: error: covariant type A occurs in invariant position in type G[A] of parameter g
// expect: 9:7: error: contravariant type A occurs in invariant position in type G[A] of method get
// expect: 2 errors found
// The arguments of a higher-kinded parameter stand where its own parameters' variances put them.
// scalac shows the second type as `[G[_$2]]: G[A]`.
trait Box[+A]:
  def put[G[_]](g: G[A]): Unit
trait Out[-A]:
  def get[G[_]]: G[A]
trait Fine[+A]:
  def get[G[+_]]: G[A]
  def put[G[-_]](g: G[A]): Unit
  def both[G[+_, -_]](g: G[Int, A]): G[A, Int]
