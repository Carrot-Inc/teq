// expect: 18:14: error: type mismatch: found Any => Int, required Int
// expect: 21:15: error: type mismatch: found Any => Int, required Int
// expect: 24:15: error: type mismatch: found Any => Any, required Int
// expect: 27:14: error: type mismatch: found [K[_]] => (x$1: K[Int]) => Int, required Int
// expect: 30:15: error: type mismatch: found Wrap[[K[_]] =>> K[Int]], required Int
// expect: 32:11: error: type mismatch: found 1, required K[A]
// expect: 33:19: error: covariant type A occurs in invariant position in type [G[_]]: G[A] of method m
// expect: 7 errors found
// A higher-kinded variable nothing bounds is maximized to its kind's top, the `[_] =>> Any` its
// parameter declares, whose application reduces to `Any` (`TypeApplications.appliedTo`): an
// eta-expanded method's parameter is `Any` as scalac shows it, never an error. A binder shows its
// kind as dotty's `decomposeLambdas` prints it (`K[_]`; scalac numbers the wildcard `_$n` by the
// unit's fresh names), and an open variable its type parameter's name.
class Wrap[F[G[_]]](val xs: List[Int])

def f[K[_]](x: K[Int]): Int = 1
val a = f
val b: Int = a
def f2[K[_]]: K[Int] => Int = null
val z = f2
val z1: Int = z
def f4[K[_]](x: K[Int]): K[Int] = x
val v = f4
val v1: Int = v

val pf: [K[_]] => K[Int] => Int = [K[_]] => (k: K[Int]) => 1
val g: Int = pf

def w = new Wrap[[K[_]] =>> K[Int]](List(8))
val w1: Int = w
def m[K[_], A](k: K[A]): Int = 1
val c = m(1)
trait F[+A] { def m[G[_]]: G[A] }
