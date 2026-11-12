// A downstream of a/ as tests/modules builds it. a/'s
// inline `e4` calls `Thread.State.valueOf`, a Java
// enum nested in a class. scalac 3.8.4 prints the
// .expected. teq's build of b/ over a/'s products is
// to accept the expansion (tests/modules' build).
package nb

import np.sub.*

@main def run(): Unit =
  println(Pk.e4("NEW"))
