// A macro's run makes the first call of a standard library method whose body holds inline calls (an array's
// `take`) while this file's calls are expanded; the next call here is a macro of the same run, so the file
// waits and is compiled again (dotty's suspended unit, `Inliner.expandMacro`). The library method is no part
// of the file: the run in the third file, in the same round, calls it as its own compilation left it.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

object Reads:
  val counted: Int = Counting.pair
  val fixed: Int = Fixing.level
