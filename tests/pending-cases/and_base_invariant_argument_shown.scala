// An invariant class argument of a member seen through
// two instances (`arr: Array[A]` from `IB & SB`) is the
// bounds of the range (`rangeToBounds`): scalac 3.8.4
// shows `Found: Array[? >: Int | String <: Int & String]`.
trait Box[A] { def arr: Array[A] }
trait IB extends Box[Int]
trait SB extends Box[String]
def t(x: IB & SB) =
  val a: Array[? <: Int] = x.arr
  val b: Array[? >: String] = x.arr
  val c: Array[? <: Boolean] = x.arr
