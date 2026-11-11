// expect: 16:57: error: Found:    (x : h.KC[Nothing])
// expect: 1 error found
// An alias's right side applying a parameter instantiated to a type lambda over an abstract contravariant member, the
// alias's other argument a wildcard (the closure adversary of the pass on 861d8acf1, round 22, row ST4 from source):
// dotty's `appliedTo` of the alias's lambda to the bounds argument takes the `Reducer`'s class arm for `F[A]` (`F`'s
// `typeSymbol` is its bound's, `Any`), passing the wildcard whole to `[X] =>> h.KC[X]`, whose `Reducer` reads `KC`'s
// contravariance (TypeApplications.scala 138-152, Types.scala 6327-6338) and gives the lower end, `h.KC[Nothing]`.
// scalac 3.8.4 rejects: Found (x : h.KC[Nothing]), Required h.KC[Int]. teq gives the same since round 22 (the
// resolver's application through the worker's `applied_to`, `ReducerView::head_symbol_is_class`). PostTyper's E043
// (`checkWildcardApply`, unreducible application to wildcard arguments), which scalac reports where typer passes, has
// no teq arm (Checking.scala is not ported in this landing).
class H:
  type KC[-X]
val h = new H
type App[F[_], A] = F[A]
def use(x: App[[X] =>> h.KC[X], ? <: Int]): h.KC[Int] = x
@main def storeReducerContraMemberAlias(): Unit = println("ok")
// expect: Required: h.KC[Int]
