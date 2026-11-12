// A file given up in two rounds running: its first macro, which reads its expansion's line and calls no
// method of the standard library, runs in the second, and the next call is a macro of that round's run.
package rounds

object Reads:
  val line: Int = Counting.spot
  val fixed: Int = Fixing.level
