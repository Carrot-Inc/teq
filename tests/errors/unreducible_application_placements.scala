// expect: 24:11: error: unreducible application of higher-kinded type [X] =>> h.KC[X] to wildcard arguments
// expect: 25:21: error: unreducible application of higher-kinded type [X] =>> h.KC[X] to wildcard arguments
// expect: 26:17: error: unreducible application of higher-kinded type [X] =>> h.KC[X] to wildcard arguments
// expect: 27:25: error: unreducible application of higher-kinded type [X] =>> h.KC[X] to wildcard arguments
// expect: 29:15: error: unreducible application of higher-kinded type [Y] =>> h.KC[Y] to wildcard arguments
// expect: 31:19: error: unreducible application of higher-kinded type [X] =>> h.KC[X] to wildcard arguments
// expect: 33:12: error: unreducible application of higher-kinded type [X] =>> h.KC[X] to wildcard arguments
// expect: 35:12: error: unreducible application of higher-kinded type F to wildcard arguments
// expect: 36:18: error: unreducible application of higher-kinded type F to wildcard arguments
// expect: 37:12: error: unreducible application of higher-kinded type [X] =>> h.KC[X] to wildcard arguments
// expect: 37:52: error: unreducible application of higher-kinded type [X] =>> h.KC[X] to wildcard arguments
// expect: 38:29: error: unreducible application of higher-kinded type F to wildcard arguments
// expect: 12 errors found
// `Checking.checkAppliedType`'s `checkWildcardApply` (Checking.scala 130-141, E043) at each place a
// written application meets it, as scalac 3.8.4 reports each (one column before the expectation's):
// an alias's right side, a `new`'s type argument, a using parameter, a parent's type argument, a
// match type's case body, a type lambda's body, a type member's right side, a class's higher-kinded
// parameter applied to `?`, an alias's, two in one signature, a parameter bounded by a class's lambda.
class H:
  type KC[-X]
val h = new H
type App[F[_], A] = F[A]
class Holder[A]
type T1 = App[[X] =>> h.KC[X], ? <: Int]
val v2 = new Holder[App[[X] =>> h.KC[X], ? <: Int]]
def f3(using x: App[[X] =>> h.KC[X], ? <: Int]): Int = 1
class K4 extends Holder[App[[X] =>> h.KC[X], ? <: Int]]
type R5[X] = X match
  case Int => App[[Y] =>> h.KC[Y], ? <: Int]
  case _ => String
type L6 = [Y] =>> App[[X] =>> h.KC[X], ? <: Y]
trait T7:
  type U = App[[X] =>> h.KC[X], ? <: Int]
class C8[F[_]]:
  def get: F[?] = ???
type Ap9[F[_]] = F[?]
def f10(x: App[[X] =>> h.KC[X], ? <: Int], y: List[App[[X] =>> h.KC[X], ? >: String]]): Int = 1
def f11[F[X] <: List[X]](x: F[?]): Int = 1
