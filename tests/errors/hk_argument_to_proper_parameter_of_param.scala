// expect: 13:18: error: type argument List does not have the same kind as its bound
// expect: 14:18: error: type argument List does not have the same kind as its bound
// expect: 17:27: error: type argument c.L does not have the same kind as its bound [_]
// expect: 3 errors found
// A higher-kinded type parameter applied to a higher-kinded argument where its own parameter is proper
// (`type L[F[_]] = F[F]`):
// scalac 3.8.4 rejects each application, `Type argument List does not have the same kind as its bound`
// (`Checking.preCheckKind`, Checking.scala 189-193, over the constructor's parameter bounds, there to keep self
// applications out, neg/i2771), a higher-kinded parameter's (`F[_]`, `HKTypeLambda.any`'s parameters) and an abstract
// type member's (`c.L` of `type L[X[_]]`) as a class's.
// scalac points at the selection's name (`c.L`'s `L`),
// the expectation at the argument.
type L[F[_]] = F[List]
def f[F[_]](x: F[List]): Int = 1
trait HasL:
  type L[X[_]]
def h(g: (c: HasL) => c.L[c.L]): HasL => Any = g
@main def hkArgumentToProperParameterOfParam(): Unit = println("ok")
