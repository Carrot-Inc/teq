// jars: sourcecode-0.4.4
// targets: js interp
//> using dep com.lihaoyi::sourcecode:0.4.4
// The macro of the run is called in a later file, not in the one whose run of the library's macro makes the
// first calls of the standard library methods: that file's round stands.
package rounds

object Reads:
  def at(using line: sourcecode.Line): Int = line.value
  val early: Int = at
