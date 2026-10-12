// A constant's type is widened where a value is inferred or a variable instantiated, and kept
// only by a `final val` (`inferredResultType`'s `isInlineVal`) or where the expected type asks for
// it; a `Float`'s literal type is its value, no other `Float` (`Constants.Constant(Float)`).
class Box[T](val value: T)
def box[T](x: T): Box[T] = new Box(x)
object C:
  final val f = 1.5f
  final def d = 7
  val v = 7
def run() =
  val f: 1.5f = 2.5f
  val g: 1.5f = 1.5f
  val h: 1.5f = C.f
  val x = 1
  val n: 1 = x
  val dd: 7 = C.d
  val vv: 7 = C.v
  val a: 1 = box((1: Int)).value
  val l: List[1] = List(x)
  val k: 2.5f = 1.5f + 1f
// expect: 11:17: error: type mismatch: found 2.5f, required 1.5f
// expect: 15:14: error: type mismatch: found Int, required 1
// expect: 16:15: error: type mismatch: found Int, required 7
// expect: 17:15: error: type mismatch: found Int, required 7
// expect: 18:14: error: type mismatch: found Int, required 1
// expect: 19:25: error: type mismatch: found Int, required 1
// absent: 12:
// absent: 13:
// absent: 20:
