// jars: rounddemand-lib
// A library's macro, the body of an inline given this file searches for, calls for the first time a library
// method whose inline call gives an erased result; the next call here is a macro of the same run, so the
// file waits and is compiled again. The library method is no part of the file: its result stays unboxed
// where its body consumes it (the unboxing of an expansion is placed after the search that asked for it),
// and the program calls it at run time.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

import scala.compiletime.summonInline

object Reads:
  inline given warmed: Int = rounddemand.Warmed.one
  inline def summoned: Int = summonInline[Int]
  val early: Int = summoned
  val fixed: Int = Fixing.level
