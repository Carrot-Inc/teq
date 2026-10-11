// An argument whose type is a constant binds the parameter to that constant where evaluating it is
// idempotent (`Inliner.computeParamBindings`: `isIdempotentPath` over `TreeInfo.exprPurity`): a literal, a
// singleton ascription, an operation over constants, and a block whose statements are pure (a val, a local
// method: `statPurity`), which takes its result's constant. tests/errors/inline_parameter_impure_path has the
// impure blocks. scalac prints the lines of the .expected file.
import scala.compiletime.constValue
inline def value(x: Int): x.type = constValue[x.type]
@main def run(): Unit =
  println(value((3: 3)))
  println(value({ val y = 2; 3 }))
  println(value({ def y: Int = 3; 4 }))
  println(value((3: Int) + 2))
  println(value(1 + 5))
