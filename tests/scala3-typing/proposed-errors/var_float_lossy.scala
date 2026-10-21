// The lossy widening of a constant against an open variable (`f[T <: Float]`'s `T`): `adaptToSubType` tests the
// expected type's `classSymbol`, which a `TypeVar` reads through its `origin`'s `paramInfo` to its upper bound, `Float`:
// E167, an error under `-Werror` as scalac 3.8.4 gives it (the review of `70fa37159`, item 2; teq gives it since the
// twelfth fix-up, `class_symbol`'s variable arm). scalac's line, at its tree's point where teq prints its start.
// expect: 9:12: warning: Widening conversion from Int to Float loses precision.
object VarFloat {
 def id[T](x:T):T=x
 def f[T <: Float](x: T): T = x
 val x = f(id[16777217](16777217))
}
