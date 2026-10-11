// expect: 12:11: error: (x : Int) is not a constant type; cannot take constValue
// expect: 13:11: error: (y : Int) is not a constant type; cannot take constValue
// expect: 2 errors found
// A constant argument is the parameter's path where its type is a literal type; an ascribed one or a
// plain inline call is typed at `Int`, which binds the parameter's proxy (`Inliner.computeParamBindings`),
// so `constValue` of the parameter's type finds no constant. scalac names the proxy `x$proxy1`.
import scala.compiletime.constValue
inline def value(x: Int): x.type = constValue[x.type]
inline def value2(y: Int): y.type = constValue[y.type]
inline def idx: Int = 2
@main def run(): Unit =
  println(value((3: Int)))
  println(value2(idx))
  println(value(3) + value2(4))
