// A method's explicit type arguments of its formals' kinds, which PostTyper's `Checking.checkBounds` accepts: a
// higher-kinded formal's bound `[_] =>> Any` (`F[_]`), a lambda bound (`F[X] <: Seq[X]`), `AnyKind` and a proper
// formal's simple kind, each argument a class, an alias, a lambda, a parameter, a member or `Nothing`.
object Ops:
  type L[X] = List[X]
  type M[X] = Map[Int, X]
  opaque type O[X] = List[X]
  def f[F[_]]: Int = 1
  def two[F[_, _]]: Int = 2
  def cov[F[+_]]: Int = 3
  def hh[F[_[_]]]: Int = 4
  def seq[F[X] <: Seq[X]]: Int = 5
  def ak[T <: AnyKind]: Int = 6
  def proper[A]: Int = 7
  class W[G[_]]
  trait Q { type C[X] }
  def k[G[_]]: Int = f[G] + cov[List]
  def q(x: Q): Int = f[x.C]
  def all: Int =
    f[List] + f[Option] + f[[X] =>> Map[Int, X]] + f[L] + f[M] + f[O] + f[Nothing] + f[Array] + f[[X] =>> Int => X] +
      two[Map] + two[Either] + two[Tuple2] + hh[W] + hh[[G[_]] =>> Int] + seq[List] + seq[Vector] + ak[List] + ak[Int] +
      ak[Map] + proper[Int] + proper[List[Int]] + proper[Nothing] + k[Option] + q(new Q { type C[X] = X })
@main def run(): Unit = println(Ops.all)
