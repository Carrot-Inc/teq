// A line that starts with an operator continues the expression of the line before when white
// space and an operand follow it, no blank line comes between, and the operand, on a later line,
// is indented at least as far as the operator (scalac's `isLeadingInfixOperator`). Otherwise the
// line is a statement of its own, which leaves the value alone.
object Main:
  extension (i: Int) def max_+(j: Int): Int = i max j
  def two(): Int = 2
  extension (sc: StringContext) def bang_!(args: Any*): Int = 2

  def backquoted: Int =
    val r = 1
      `max` 2
    r

  def ownLine: Int =
    val r = 1
      +
      2
    r

  def ownLineDeeper: Int =
    val r = 1
      +
        (2)
    r

  def tab: Int =
    val r = 1
      +	2
    r

  def lineComment: Int =
    val r = 1
      + // the operand follows
      2
    r

  def blockComment: Int =
    val r = 1
      + /* two */ 2
    r

  def backquotedUnaryOperand: Int =
    val `+` = 2
    val r = 1
      + `+`
    r

  def interpolationOperand: Int =
    val r = 1
      + bang_!""
    r

  def unaryOperand: Int =
    val r = 1
      - -1
    r

  def notUnaryOperand: Int =
    val r = 2
      * -3
    r

  def endsInOperator: Int =
    val r = 1
      max_+ 2
    r

  def atEnclosingWidth: Int =
    val r =
      1
    + 2
    r

  def betweenWidths: Int =
    val r =
        1
      + 2
    r

  def inBraces: Int = {
    val r = 1
      `max` 3
    r
  }

  def connective: Boolean =
    val a = true
    a
      && !false

  // The indented body of a colon argument's lambda ends where it does: the operators after it
  // continue the call, the ones inside it the body.
  def afterColonLambda(errors: Set[Int], a: Boolean, b: Boolean): Boolean =
    errors.exists: error =>
      error > 1 && error < 5
    || a
    || b

  def insideColonLambda(errors: Set[Int]): Boolean =
    errors.exists: error =>
      error > 10
      || error < 0

  def blankLine: Int =
    val r = 1

      + two()
    r

  def noSpace: Int =
    val r = 1
      +two()
    r

  def backquotedOperand: Int =
    val r = 1
      + `two`()
    r

  def backquotedThenOperator: Int =
    var x = 1
    two()
    `x` += 1
    x

  def afterEndMarker(): Unit =
    def f: Int =
      1
    end f
    + { println("effect"); 2 }
    println("before call")
    println(f)

  def main(args: Array[String]): Unit =
    afterEndMarker()
    println(List(backquoted, ownLine, ownLineDeeper, tab, lineComment, blockComment))
    println(List(unaryOperand, notUnaryOperand, endsInOperator, atEnclosingWidth, betweenWidths, inBraces))
    println(List(backquotedUnaryOperand, interpolationOperand))
    println(connective)
    println(List(afterColonLambda(Set(), false, true), afterColonLambda(Set(3), false, false), afterColonLambda(Set(7), false, false)))
    println(List(insideColonLambda(Set(11)), insideColonLambda(Set(-1)), insideColonLambda(Set(5))))
    println(List(blankLine, noSpace, backquotedOperand, backquotedThenOperator))
