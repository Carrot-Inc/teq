// A downstream of a/ as tests/modules builds it. a/'s `e5` returns a
// Java enum nested in a class, `Thread.State`. scalac 3.8.4 prints the
// .expected. b/'s pickle and classes are to be the same in teq's whole
// build and in its build over a/'s products (tests/modules' classes,
// subset), and so its check-build pickle and its bundles (check, bundle).
package nb

import np.sub.*

@main def run(): Unit =
  println(Pk.e5("RUNNABLE"))
