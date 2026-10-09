// A lazy val passed as an argument after a default: its read is evaluated where the source has
// it, before the later arguments and the default (scalac's `isPureExpr` holds it idempotent, not
// pure).
object DefaultsLazyArgument {
  def mark(s: String): Int = { print(s); 0 }
  def f(a: Int = mark("D"), b: Int, c: Int): Unit = ()
  def main(args: Array[String]): Unit = {
    lazy val l: Int = mark("L")
    f(b = l, c = mark("C"))
    println()
  }
}
