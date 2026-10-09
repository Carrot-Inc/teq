// A line between two indentation widths that cannot start a statement continues the statement
// out of the inner region; in the body of a brace lambda no width is checked.
object O:
  def f(x: Int): Int =
    val y = if x > 0 then
        x + 1
      else
        x - 1
    y
  def run(g: Int => Int): Int = g(1)
  val r = run { x =>
    val y = if x > 0 then
        x + 1
      else
        x - 1
    y
  }
  val s: String = f(1)
