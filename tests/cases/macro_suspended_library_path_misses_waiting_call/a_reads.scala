// jars: rounddemand-lib
// A library's macro runs a library method for the first time while this file's calls are expanded. One of the
// method's two inline calls is plain and is expanded; the other is the library's own macro, which reads an
// object this program defines too, in a file that waits for a macro of the run, so this file waits as well.
// Compiled again, the macro's run takes the branch of the plain call, which stays as the method's own
// compilation left it.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

object Reads:
  val early: Int = rounddemand.Chosen.one
