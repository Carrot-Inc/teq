// An `else` between the widths of a brace lambda's body and its `then` branch continues the `if`:
// scalac checks no indentation width in a brace block.
object ElseBetweenWidths:
  def run(f: Int => Int): Int = f(1)
  val r = run { x =>
    val y = if x > 0 then
        x + 1
      else
        x - 1
    y
  }
  @main def main(): Unit = println(r)
