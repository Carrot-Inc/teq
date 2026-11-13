// command: build
// expect: y_sizes.scala:5:41: error: the inline call rounds.Sizes.two in rounds.Sizes.sized was not expanded
// A method's default argument is a plain inline call. The default's getter is compiled when another file's
// call first needs it, inside that file's expansion, which is then given up for a macro of the run; compiled
// again, the file finds the getter made and its inline call is left as a call, so the output would hold a
// call of a method nothing defines. scalac prints `2 42 2 2`.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

object Reads:
  val first: Int = Sizes.sized(Array(1, 2, 3))
  val second: Int = Fixing.answer
  val third: Int = Sizes.sized(Array(1, 2, 3, 4))
