// The F-bound over a wildcard outside the bound's own completion: `BS[?, S]` in a parameter type, with `S`
// completed as `S <: BS[Int, S]`. `BS`'s `S <: BS[T, S]` is seen as `BS[BS[?, S]#T, S]` (Checking.scala 118,
// Types.scala 2794). `S` is in that bound as the `LazyRef` of `BS`'s own F-bound (`lazyRefs`, TypeOps.scala 651-653),
// but `massage` of its bound `BS[Int, S]`, which has no
// wildcard, changes nothing, so no GADT bound is installed (`narrowBound`, 676), and `BS[Int, S] <: BS[<app>#T, S]`
// fails. scalac 3.8.4 rejects it (E057):
//   11:32 Type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// teq is to reject it as scalac does; substituting the wildcard (`BS[Int, S] <: BS[?, S]`) accepts it.
// (scala3's tests/pos/i6146.scala, `S <: BS[?, S]`, whose bound `massage` skolemizes, is accepted by scalac.)
trait BS[T, S <: BS[T, S]]
def f[S <: BS[Int, S]](x: BS[?, S]): Int = 1
// expect: 11:33: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 1 error found
