// A variable a type-level operation holds is not forced at its application: `T` of `mk(1)`,
// nonvariant in `Box[T]`, stays open for the selection, which the expected `1` settles, and the
// operation `T + 1` reduces once `T` is known (dotty leaves invariant occurrences open and reduces
// an operation over instantiated arguments).
import scala.compiletime.ops.int.+
class Box[A](val x: A)
def mk[T <: Int](x: T): (Box[T], T + 1) = (new Box(x), (x + 1).asInstanceOf[T + 1])
@main def run() =
  val x: 1 = mk(1)._1.x
  println(x)
  val p = mk(4)
  println(p._2)
