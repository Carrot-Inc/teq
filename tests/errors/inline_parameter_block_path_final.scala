// expect: 9:25: error: (x$proxy1 : Int) is not a constant type; cannot take constValue
// expect: 1 error found
// A block ending in a final val's reference is no path either: the inliner binds the parameter's proxy before
// FirstTransform takes the reference to its literal (dotty's Inlining before FirstTransform), so `constValue` finds
// no constant, as for a non-final val (inline_parameter_block_path).
import scala.compiletime.constValue
inline def value(x: Int): x.type = constValue[x.type]
final val b: 9 = 9
def inBlockFinal: Int = value({ def unused = 0; b })
