// expect: 10:20: error: (x : Int) is not a constant type; cannot take constValue
// expect: 1 error found
// A block is no path (`TreeInfo.exprPurity` meets its result's purity with its statements', which are no
// paths): one ending in a path, whose type is the path's singleton and no constant type, binds the
// parameter's proxy (`Inliner.computeParamBindings`), so `constValue` finds no constant, whatever the
// path's own type. scalac names the proxy `x$proxy1`.
import scala.compiletime.constValue
inline def value(x: Int): x.type = constValue[x.type]
val a: 8 = 8
def inBlock: Int = value({ def unused = 0; a })
