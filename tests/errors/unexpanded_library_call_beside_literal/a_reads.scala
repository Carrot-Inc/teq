// jars: rounddemand-lib
// command: build
// expect: error: the inline call rounddemand.Derived.derived in rounddemand.Reader.settled was not expanded
// A library's macro runs a library method whose one branch is a literal and whose other is a call of the
// library's own macro, which reads an object this program defines too, in a file that waits for a macro of
// the run: the call is left as a call. The program calls the method on that branch, so the output would hold
// a call of a method nothing defines. scalac, to which the method is the jar's bytecode, prints `13 7 13`.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

object Reads:
  val early: Int = rounddemand.Settled.one
