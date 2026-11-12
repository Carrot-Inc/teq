// jars: sourcecode-0.4.4
// targets: js interp
//> using dep com.lihaoyi::sourcecode:0.4.4
// As macro_suspended_after_library_demand, with no later run of the library's macro in another file: the
// program itself calls the standard library methods the macro's run demanded (an array's `take`, a string's
// `linesIterator`), and the built JavaScript runs them.
package rounds

object Reads:
  def at(using line: sourcecode.Line): Int = line.value
  val early: Int = at
  val fixed: Int = Fixing.level
