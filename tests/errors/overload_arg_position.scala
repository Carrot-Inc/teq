// An argument overload resolution typed ahead is adapted to the chosen alternative's parameter at its own
// place (dotty's `FunProto.typedArgs` adapted per argument tree): `Array("s")` against `Array[Int]` takes the
// generic `apply` with `T` of `Int`, and the mismatch stands at `"s"`, in a splice too.
def f(xs: Int*) = xs.size
val x = f(Array("s")*)
val w: Array[Int] = Array("s")
object O:
  def ap[T](xs: T*): Array[T] = ???
  def ap(x: Int, xs: Int*): Array[Int] = ???
val v: Array[Int] = O.ap(1, "s")
// expect: overload_arg_position.scala:5:17: error: type mismatch: found "s", required Int
// expect: overload_arg_position.scala:6:27: error: type mismatch: found "s", required Int
// expect: overload_arg_position.scala:10:29: error: type mismatch: found "s", required Int
// expect: 3 errors found
