// jars: sourcecode-0.4.4
// targets: js interp
//> using dep com.lihaoyi::sourcecode:0.4.4
// Another file uses the library's macro before the file that is given up: the first calls of the standard
// library methods are made in a round that stands.
package rounds

object Later:
  val again: Int = Reads.at
