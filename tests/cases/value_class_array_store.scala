// An `Array[V]` of a value class stores the box (dotty's `Erasure.Boxing.box`): no `ArrayStoreException`, the
// design pass's witness of TASKS 243, closed on master.
class V(val x: Int) extends AnyVal
object Api:
  def f(x: Array[V], v: V): Array[V] = { x(0) = v; x }
@main def run(): Unit = println(Api.f(Array(new V(1)), new V(2))(0).x)
