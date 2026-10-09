// A clause's defaults are evaluated with the clause, before the next clause's arguments.
object DefaultsClauses {
  def mark(s: String): Int = { print(s); 0 }
  def f(a: Int = mark("D"), b: Int)(c: Int): Unit = ()
  def main(args: Array[String]): Unit = {
    f(b = mark("B"))(mark("C"))
    println()
  }
}
