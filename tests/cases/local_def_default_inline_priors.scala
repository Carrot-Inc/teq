// A local inline def's later default read where an inline parameter's argument or default stands in its place: the
// earlier argument is the inline parameter's tree, not a path, so the default's getter is applied to it evaluated
// first into a temporary; each tree is evaluated once, in the order written.
object Main {
  var log = List.empty[String]
  def note(s: String, v: Int): Int = { log = log :+ s; v }

  def run(k: Int): Int = {
    inline def p(inline a: Int = note("default-a", 1))(b: Int = a + k): Int = a * 10 + b
    inline def q(inline a: Int, inline c: Int = note("default-c", 2))(b: Int = a + c): Int = a + b + c
    val r1 = p()()
    val r2 = p(note("given-a", 3))()
    val r3 = q(note("q-a", 4))()
    val r4 = q(note("q-a2", 5), note("q-c", 6))()
    val after = note("after", 7)
    r1 + r2 + r3 + r4 + after
  }

  def main(args: Array[String]): Unit = {
    println(run(5))
    println(log.mkString(","))
  }
}
