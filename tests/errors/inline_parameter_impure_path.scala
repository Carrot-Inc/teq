// expect: 12:20: error: (x : Int) is not a constant type; cannot take constValue
// expect: 13:23: error: (y : Int) is not a constant type; cannot take constValue
// expect: 2 errors found
// An argument whose type is a constant binds the parameter to that constant only where evaluating it
// is idempotent (`Inliner.computeParamBindings`: `isIdempotentPath` over `TreeInfo.exprPurity`): a block
// with a `var` or an effect among its statements is not (`statPurity`), and binds the parameter's proxy,
// so `constValue` of the parameter's type finds no constant. A block of a val and of a local method
// is, and gives its result's constant (tests/cases/inline_parameter_pure_block).
import scala.compiletime.constValue
inline def value(x: Int): x.type = constValue[x.type]
inline def value2(y: Int): y.type = constValue[y.type]
def withVar: Int = value({ var y = 2; 3 })
def withEffect: Int = value2({ println("effect"); 3 })
