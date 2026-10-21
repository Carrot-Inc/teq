// `CheckNonCyclicMap` over an applied abstract member by its parameters' variances (`mapOver(AppliedType)`'s
// `mapArgs(args, tyconTypeParams(tp))`, each `paramVarianceSign` into `mapArg`; the review of `cf6cad6c1`, item 3):
// a covariant member's argument `List[B]` in a lower bound is a nested cycle allowed, a class's member and an
// object's alike (an invariant one is E140, `tests/errors/binder_cycle_map_branches.scala`'s kind).
trait P { type F[+X] }
object Q { type F[+X] }
type L = [B >: P#F[List[B]]] =>> Int
type M = [B >: Q.F[List[B]]] =>> Int

@main def run(): Unit =
  val l: L[Any] = 1
  val m: M[Any] = 2
  println(s"$l $m")
