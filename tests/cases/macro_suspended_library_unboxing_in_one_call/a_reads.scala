// jars: rounddemand-lib
// As macro_suspended_library_unboxing, with the search for the inline given and the macro of the run in one
// call: the file is given up before the unboxing of the library method's expansion, which the search left
// for after itself, is placed. The method is no part of the file: its result is unboxed where its body
// consumes it all the same, and the program calls it at run time.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

import scala.compiletime.summonInline

object Reads:
  inline given warmed: Int = rounddemand.Warmed.one
  inline def both: Int = summonInline[Int] + Fixing.level
  val early: Int = both
