// A downstream of a/ as tests/modules builds it. a/'s `e5` returns a Java enum nested in a class, `Thread.State`.
// scalac 3.8.4 prints the .expected, and so do teq's runs. b/'s pickle and classes differ between the whole build and
// the build over a/'s products (tests/modules' classes, subset), and so do its check-build pickle and its bundles
// (check, bundle); reviewer #14's census gives b/'s body withheld over the products, "the erasure of a Blocked in a
// signature". The same before the SELECTtpt port (tip34). Found by reviewer #14 of the workers exports helper (push 9).
package nb

import np.sub.*

@main def run(): Unit =
  println(Pk.e5("RUNNABLE"))
