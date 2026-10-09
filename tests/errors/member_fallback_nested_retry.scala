// expect: 13:29: error: type mismatch: found String, required Int
// expect: 13:36: error: not found: undefinedName
// A member call an extension takes over through the retry on the qualifier, whose later clause
// holds a call that fails in its first clause: that call is no part of the retry, so the list
// past its failing clause is typed alone, once, for its own error (`undefinedName`), as dotty's
// `realApply` types an erroneous application's outer arguments (Applications.scala 1438). The
// deferral of such lists belongs to the application the retry makes, not to its arguments.
def g(a: Int)(b: String): String = b
class C { def f(a: Int)(b: String): String = b }
extension (c: C) def f(a: String, n: Int)(b: String): String = b
object Main:
  def main(args: Array[String]): Unit =
    println(C().f("s", 0)(g("bad")(undefinedName)))
