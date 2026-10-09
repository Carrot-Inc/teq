// expect: 10:17: error: expansion failed
// expect: 11:18: error: expansion failed
// expect: 2 errors found
// A constant narrowing of a pending call whose expansion fails, negated or not, narrows by the
// call's type: the expansion's error alone is reported, as scalac and master report it.
object M:
  inline def bad: 1 = scala.compiletime.error("expansion failed")

@main def run(): Unit =
  val b: Byte = M.bad
  val c: Byte = -M.bad
  println(b + c)
