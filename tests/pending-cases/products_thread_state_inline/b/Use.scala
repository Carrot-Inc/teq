// A downstream of a/ as tests/modules builds it. a/'s inline `e4` calls `Thread.State.valueOf`, a Java enum nested in a
// class. scalac 3.8.4 prints the .expected. b/'s build over a/'s products refuses the expansion: "not supported yet:
// value State of Pk (needed by np.sub.Pk.e4)" (tests/modules' build); the pickled `Thread.State` is read as a member of
// the enclosing object. The same before the SELECTtpt port (tip34). Found by reviewer #14 of the workers exports helper
// (push 9).
package nb

import np.sub.*

@main def run(): Unit =
  println(Pk.e4("NEW"))
