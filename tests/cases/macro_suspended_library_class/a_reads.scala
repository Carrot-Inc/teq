// jars: rounddemand-lib
// A macro's run makes the first instance of a library class whose template holds an inline call (an array's
// literal) while this file's calls are expanded; the next call here is a macro of the same run, so the file
// waits and is compiled again. The class is no part of the file: the run in the third file, in the same
// round, constructs it as its own check left it.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

object Reads:
  val counted: Int = Counting.tally
  val fixed: Int = Fixing.level
