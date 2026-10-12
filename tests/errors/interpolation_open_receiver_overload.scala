// A receiver whose application leaves its class's parameter open (`RBox(2)`: `T` is nonvariant in
// `RBox[T]`, `Inferencing.interpolateTypeVars`) keeps it open for the selection, so both `put(x: T)`
// and `put(x: Any)` apply to a `String` argument and neither is more specific, as scalac reports;
// a receiver of a fixed type (`RBox[Int](2)`, a stored `RBox(2)`) chooses `put(x: Any)`.
import scala.annotation.targetName
class RBox[T](val value: T):
  def put(x: T): String = "put(T) " + x
  @targetName("putAny") def put(x: Any): String = "put(Any) " + x
def run() =
  val a = RBox(2).put("x")
  val b = RBox[Int](2).put("x")
  val stored = RBox(2)
  val c = stored.put("y")
// expect: 10:11: error: Ambiguous overload. The overloaded alternatives of method put in class RBox with types
// absent: 11:11:
// absent: 13:11:
