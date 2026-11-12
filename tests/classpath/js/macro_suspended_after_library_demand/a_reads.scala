// jars: sourcecode-0.4.4
// targets: js interp
//> using dep com.lihaoyi::sourcecode:0.4.4
// A library's macro (sourcecode's line, an implicit argument) makes the first calls of standard library
// methods whose bodies hold inline calls while this file's calls are expanded; the next call is a macro of
// the same run, so the file waits and is compiled again. The third file's use of the implicit, in the same
// round, runs the library's macro over those methods as their own compilation left them.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

object Reads:
  def at(using line: sourcecode.Line): Int = line.value
  val early: Int = at
  val fixed: Int = Fixing.level
