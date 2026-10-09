// A def missing its `:` and its `=`: what is left of the line is skipped with the header, and no
// body is guessed from the result type it meant.
object O:
  def h(x: Int) Int (x + 1)
  val s: String = h(1)
  val bad: String = 2
