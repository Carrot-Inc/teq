// jars: rounddemand-lib
// command: build
// expect: error: the inline call rounddemand.Derived.derived in rounddemand.Reader.either was not expanded
// The method of `unexpanded_library_call_beside_inline`, called by the program on the branch of its plain
// inline call: the run never takes the branch that holds the call left as a call, and the build is refused
// all the same, since the method written would hold a call of a method nothing defines whatever a run
// takes. scalac prints `13 7 13`.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

object Reads:
  val early: Int = rounddemand.Chosen.one
