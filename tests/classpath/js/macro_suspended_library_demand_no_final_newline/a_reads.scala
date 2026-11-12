// jars: sourcecode-0.4.4
// targets: js interp
//> using dep com.lihaoyi::sourcecode:0.4.4
// As macro_suspended_after_library_demand, where this file ends without a line end: the library's macro reads
// the file's lines by another standard library method.
package rounds

object Reads:
  def at(using line: sourcecode.Line): Int = line.value
  val early: Int = at
  val fixed: Int = Fixing.level