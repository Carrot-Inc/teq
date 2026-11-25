// A local def's defaults after earlier arguments that are not paths: a call, a block, a by-name argument, named
// arguments out of order. A default reads the parameters of the clauses before its own; each earlier argument is
// evaluated once, in the order written, before the default's getter reads it.
object Main {
  var log = List.empty[String]
  def note(s: String, v: Int): Int = { log = log :+ s; v }

  def run(k: Int): Int = {
    def f(a: Int)(b: Int = a + k): Int = a * 10 + b
    def g(a: Int, b: => Int)(c: Int = a + b): Int = a + b + c
    def h(x: Int, y: Int)(z: Int = x - y): Int = x * 100 + y * 10 + z
    val r1 = f(note("call", 3))()
    val r2 = f({ val t = note("block", 4); t + 1 })()
    val r3 = g(note("first", 1), note("by-name", 2))()
    val r4 = h(y = note("y", 2), x = note("x", 7))()
    val after = note("after", 9)
    r1 + r2 + r3 + r4 + after
  }

  def main(args: Array[String]): Unit = {
    println(run(5))
    println(log.mkString(","))
  }
}
