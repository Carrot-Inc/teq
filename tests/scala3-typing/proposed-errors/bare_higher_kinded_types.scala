// A higher-kinded type where a proper one is wanted outside a binder's completion: `Namer.valOrDefDefSig`'s
// `checkSimpleKinded` (E056 `Missing type parameter for`, a `val`'s, a parameter's, a named function's parameter's
// type) and `Checking.preCheckKind` over `=>`'s, a tuple's, `&`'s and `|`'s arguments. teq checks a class's and a lambda's
// application alone (master's; the last line's is teq's today); a binder's completion keeps both verdicts (the review of
// `cf6cad6c1`, items 1 and 2). scalac 3.8.4 gives the eight, a kind error at its tree's point where teq prints its start.
// expect: 15:8: error: Missing type parameter for List
// expect: 16:10: error: Missing type parameter for Option
// expect: 17:10: error: type argument List does not have the same kind as its bound
// expect: 18:11: error: type argument List does not have the same kind as its bound
// expect: 19:10: error: type argument Option does not have the same kind as its bound
// expect: 20:10: error: type argument List does not have the same kind as its bound
// expect: 21:12: error: Missing type parameter for List
// expect: 22:10: error: type argument List does not have the same kind as its bound
class C[A]
val x: List = Nil
def f(y: Option): Int = 1
type T = List => Int
type U = (List, Int)
type V = Option & Int
type W = List | Int
val g: (x: List) => Int = ???
val c: C[List] = ???
