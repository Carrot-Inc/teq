// A syntax error at the offset right after the application (`Aux[T, Int]]`'s second `]`) is logged at a position the
// E023's span covers (`UniqueMessagePositions.isHidden` reads `pos.start to pos.end`, the end included): scalac 3.8.4
// reports the parse error alone (`end of toplevel definition expected but ']' found` at the second `]`; the E023 shows
// with `-Yshow-suppressed-errors`). teq reports the stray `]` ("unmatched closing bracket")
// and no E023 beside it.
// The parser's message is not asserted here, its place is.
// scalac's columns are 0-based.
trait Validate[T, P] { type R }
object V:
  type Aux[T, P, R0] = Validate[T, P] { type R = R0 }
  type Few[T] = Aux[T, Int]]
// expect: wrong_number_of_type_args_under_a_syntax_error_at_its_end.scala:11:28: error
// absent: Not enough type arguments
// expect: 1 error found
