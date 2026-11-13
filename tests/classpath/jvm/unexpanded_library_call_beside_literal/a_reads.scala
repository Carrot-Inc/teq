// jars: rounddemand-lib
// The program of tests/errors/unexpanded_library_call_beside_literal on the JVM, where a jar's method is the
// jar's bytecode and no call of it is left to finish: the program prints what scalac's does.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

object Reads:
  val early: Int = rounddemand.Settled.one
