// expect: 13:12: error: unreducible application of higher-kinded type [X] =>> h.KC[X] to wildcard arguments
// expect: 14:29: error: unreducible application of higher-kinded type [X] =>> h.KC[X] to wildcard arguments
// expect: 2 errors found
// An alias applying a type lambda over an abstract contravariant member to a wildcard (the closure adversary's s22,
// round 22): dotty's typer reduces `App[[X] =>> h.KC[X], ? <: Int]` to `h.KC[Nothing]`, so both definitions type;
// PostTyper's `Checking.checkAppliedType` then re-applies the constructor under `Mode.AllowLambdaWildcardApply`
// (Checking.scala 130-141), whose `Reducer` keeps the parameter it cannot consume, and reports the unreduced result's
// `isUnreducibleWild` (E043) at both trees. scalac 3.8.4 reports both at 12:11 and 13:28; teq the same since round 23.
class H:
  type KC[-X]
val h = new H
type App[F[_], A] = F[A]
def use(x: App[[X] =>> h.KC[X], ? <: Int]): h.KC[Nothing] = x
def back(x: h.KC[Nothing]): App[[X] =>> h.KC[X], ? <: Int] = x
@main def unreducibleWildcardApplication(): Unit = println("ok")
