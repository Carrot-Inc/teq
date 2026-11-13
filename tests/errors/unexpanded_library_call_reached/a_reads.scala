// jars: rounddemand-lib
// command: build
// expect: error: the inline call rounddemand.Derived.derived in rounddemand.Reader.depth was not expanded
// A library's macro calls, on its first run alone, a library method whose body is a call of the library's own
// macro, which reads an object this program defines too, in a file that waits for a macro of the run. The
// method's body is compiled while that file is not ready and its call is left as a call; the program calls
// the method, so the output would hold a call of a method nothing defines. scalac, to which the method is the
// jar's bytecode, prints `1 7 13`.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

object Reads:
  val early: Int = rounddemand.Once.one
