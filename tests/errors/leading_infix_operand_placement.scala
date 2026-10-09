// expect: 10:5: error: expected an expression, found new line
// expect: 16:7: error: expected an expression, found new line
// expect: 20:5: error: expected end of statement, found identifier
// A line that starts with an operator whose operand stands further left, or past a blank line,
// is a statement of its own; `try` cannot be the operand of an infix operator.
object Main:
  def shallower: Int =
    val r = 1
      +
    2
    r
  def blankLine: Int =
    val r = 1
      +

      2
    r
  def tryOperand: Boolean =
    true
    && // the operand is a `try`
    try java.lang.Boolean.valueOf("true")
    finally ()
