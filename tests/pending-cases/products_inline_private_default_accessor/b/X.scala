// A downstream of a/ as tests/modules builds it. a/'s inline method calls a
// private method with and without its default. scalac 3.8.4 makes an inline
// accessor for each private member the body reads, the getter's included:
// its O$ holds `inline$pd` and `inline$pd$default$1` (PrepareInlineable),
// and pd and the getter stay private. teq's runs are to print the
// .expected, scalac's. teq's products of a/ are to be ones scalac types
// b/ over (tests/modules' scalac-b, scalac-run, scalac-lean-b). The whole
// build's classes are to be the split build's (classes, subset, bundle).
package pdgu

@main def run(): Unit =
  println(pdg.O.usePd)
