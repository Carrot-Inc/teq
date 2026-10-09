// A plain inline call given to an inline parameter of a call expanded at once (a transparent
// method's, a transparent extension's): the later expansion phase expands it before the
// parameter's copies are made, so no copy keeps a call of an inline
// method, which has no body to run.
object M:
  inline def one: Int = 1
  transparent inline def twice(inline x: Int): Int = x + x
  inline def plainTwice(inline x: Int): Int = x + x

extension (m: M.type) transparent inline def twiceOf(inline x: Int): Int = x * 2

@main def run(): Unit =
  println(M.twice(M.one))
  println(M.plainTwice(M.one))
  println(M.twiceOf(M.one))
