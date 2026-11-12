// expect: 16:57: error: Found:    (x : h.KC[Nothing])
// expect: 1 error found
// An alias's right side applying a parameter instantiated to a type lambda over an abstract contravariant member, the
// alias's other argument a wildcard:
// dotty's `appliedTo` of the alias's lambda to the bounds argument takes the `Reducer`'s class arm for `F[A]` (`F`'s
// `typeSymbol` is its bound's, `Any`), passing the wildcard whole to `[X] =>> h.KC[X]`, whose `Reducer` reads `KC`'s
// contravariance (TypeApplications.scala 138-152, Types.scala 6327-6338) and gives the lower end, `h.KC[Nothing]`.
// scalac 3.8.4 rejects: Found (x : h.KC[Nothing]), Required h.KC[Int];
// teq is to give the same. Where typer passes, scalac reports
// PostTyper's E043 (`checkWildcardApply`, unreducible application
// to wildcard arguments).
class H:
  type KC[-X]
val h = new H
type App[F[_], A] = F[A]
def use(x: App[[X] =>> h.KC[X], ? <: Int]): h.KC[Int] = x
@main def storeReducerContraMemberAlias(): Unit = println("ok")
// expect: Required: h.KC[Int]
