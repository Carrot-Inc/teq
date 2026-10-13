// The design pass's witnesses of lines the task list still held, which teq already rejects as scalac does: a
// `given` without a body outside a class, `Tuple.unapply` of a non-empty tuple, a `Float` literal type shown
// with the JVM's digits.
import scala.reflect.ClassTag
def f[A]: Unit =
  given tag: ClassTag[A]
  ()
def g = Tuple.unapply((1, 2))
val x: 1 = 3.4028235e38f
// expect: inference_leftovers_rejected.scala:6:9: error: Declaration of given instance tag not allowed here: only classes can have declared but undefined members
// expect: inference_leftovers_rejected.scala:8:23: error: type mismatch: found (Int, Int), required EmptyTuple
// expect: inference_leftovers_rejected.scala:9:12: error: type mismatch: found 3.4028235E38f, required 1
// expect: 3 errors found
