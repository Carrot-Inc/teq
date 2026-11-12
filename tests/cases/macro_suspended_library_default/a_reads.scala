// jars: rounddemand-lib
// A macro's run calls a library method with its default argument, which holds an inline call, for the first
// time while this file's calls are expanded; the next call here is a macro of the same run, so the file
// waits and is compiled again. The default is the library's: the run in the third file, in the same round,
// and the program at run time read it as its own completion left it.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

object Reads:
  val counted: Int = Counting.sized
  val fixed: Int = Fixing.level
