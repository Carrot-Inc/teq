// A `final val` initialised by a call has no constant type (dotty's `Namer` infers the call's type), so an
// `inline if` over it cannot reduce, as under scalac.
object M:
  def one: Int = 1
final val y = M.one
inline def h = inline if y == 1 then 1 else 2
val z = h
// expect: inline_if_final_val_call.scala:7:9: error: Cannot reduce `inline if` because its condition is not a constant value: y == 1
// expect: 1 error found
