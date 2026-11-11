// expect: 14:13: error: type argument Int does not conform to upper bound [_] =>> Any
// expect: 15:13: error: type argument Map does not conform to upper bound [_] =>> Any
// expect: 16:13: error: Missing type parameter for List
// expect: 17:13: error: Missing type parameters for Map
// expect: 4 errors found
// A method's explicit type arguments are checked by PostTyper's `Checking.checkBounds` (Checking.scala 72-80): an
// argument of no simple kind for a formal whose bound is no type lambda is `Missing type parameter(s) for ..` (E141); a
// higher-kinded formal's bound, `[_] =>> Any` of `F[_]`, holds an argument of another kind (E057). `Nothing`, a type
// constructor, a lambda, a higher-kinded parameter and an application of a higher-kinded member are of its kind.
trait P { type F[X] <: [Y] =>> Any }
object T:
  def f[F[_]]: Int = 1
  def g[A]: Int = 2
  def a = f[Int]
  def b = f[Map]
  def c = g[List]
  def d = g[Map]
  def e = f[Nothing] + f[List] + f[[X] =>> Option[X]] + f[P#F[Int]]
  def k[G[_]] = f[G]
@main def run(): Unit = println(T.e)
