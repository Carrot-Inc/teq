// A line that begins with `(`, `[` or `{` and stands further in than its region goes on with the line before it:
// an argument list, a type argument list, a block argument, a comment line between them or not. After a blank
// line, also one inside a block comment, and at the region's own width the line begins a statement. scalac prints
// the same lines.
object Calls:
  def twice(x: Int): Int = x * 2
  def pick[T](x: T): T = x
  def run(body: => Int): Int = body + 1

  def argument: Int = twice
    (20)

  def typeArgument: Int = pick
    [Int](3)

  def blockArgument: Int = run
    { 4 }

  def commentBetween: Int = twice
    // a comment line separates nothing
    (5)

  def afterBlankLine: Int =
    val f = twice

    (6)

  def afterBlankLineInComment: Int =
    val f = twice
    /* a comment

       holding a blank line */
    (7)

  def sameWidth: Int =
    val f = twice
    (8)

@main def main(): Unit =
  println((Calls.argument, Calls.typeArgument, Calls.blockArgument, Calls.commentBetween))
  println((Calls.afterBlankLine, Calls.afterBlankLineInComment, Calls.sameWidth))
