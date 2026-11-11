// A syntax error at the offset right after the application (`Aux[T, Int]]`'s second `]`) is logged at a position the
// E023's span covers (`UniqueMessagePositions.isHidden` reads `pos.start to pos.end`, the end included): scalac 3.8.4
// reports the parse error alone (`end of toplevel definition expected but ']' found` at the second `]`; the E023 shows
// with `-Yshow-suppressed-errors`). teq's lexer drops the stray `]` ("unmatched closing bracket"), which the typer read
// no record of, so it reported both; the unit's syntax error places (`SyntaxNotes::syntax_errors`) now hide the E023
// there. The parser's message is not asserted here, its place is. Master reports the parse error alone (no E023 for an
// alias). The closure adversary of the pass on 97f0487ac (round 24). scalac's columns are 0-based.
trait Validate[T, P] { type R }
object V:
  type Aux[T, P, R0] = Validate[T, P] { type R = R0 }
  type Few[T] = Aux[T, Int]]
// expect: wrong_number_of_type_args_under_a_syntax_error_at_its_end.scala:11:28: error
// absent: Not enough type arguments
// expect: 1 error found
