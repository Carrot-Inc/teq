// A syntax error elsewhere in the file (line 13) does not hide
// `typedAppliedTypeTree`'s E023 at line 11, whose span no earlier
// error logged (`UniqueMessagePositions`): scalac 3.8.4 reports
// the E023 and hides the argument's `Not found: type Undef` inside
// its span. The parser's message and the overload error after it
// (E051) are not asserted here. scalac's columns are 0-based.
trait Validate[T, P] { type R }
object V:
  type Aux[T, P, R0] = Validate[T, P] { type R = R0 }
object U:
  type Few[T] = V.Aux[T, Undef]
object W:
  val x: Int = (1 +
// expect: wrong_number_of_type_args_beside_a_syntax_error.scala:11:17: error: Not enough type arguments for V.Aux[T, P, R0]
// absent: Undef not found
