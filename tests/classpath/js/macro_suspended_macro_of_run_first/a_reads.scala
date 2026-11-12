// jars: sourcecode-0.4.4
// targets: js interp
//> using dep com.lihaoyi::sourcecode:0.4.4
// The macro of the run comes first in the file, before the use of the library's macro: the file is given up
// before any run, and nothing was demanded in the round.
package rounds

object Reads:
  def at(using line: sourcecode.Line): Int = line.value
  val fixed: Int = Fixing.level
  val early: Int = at
