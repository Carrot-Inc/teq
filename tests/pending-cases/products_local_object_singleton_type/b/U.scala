// A downstream of a/ as tests/modules builds it. scalac 3.8.4 prints the .expected, and so do teq's JVM runs. teq's
// products withhold `local`'s body, whose result is a tuple of its local objects ("the singleton type of the local
// L"), so the Scala.js and interpreter builds over the lean products stop on it (js-run, interp-run). The same on
// integration a097de53d9. Found beside the module constructors' flags, push 10 of the workers exports helper.
package lob

@main def run(): Unit =
  println(loa.local().toString.nonEmpty)
