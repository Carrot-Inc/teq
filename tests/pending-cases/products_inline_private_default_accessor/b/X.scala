// A downstream of a/ as tests/modules builds it. a/'s inline method calls a private method with and without its
// default. scalac 3.8.4 makes an inline accessor for each private member the body reads, the getter's included: its
// O$ holds `inline$pd` and `inline$pd$default$1` (PrepareInlineable), and pd and the getter stay private. teq's
// products of a/ hold `inline$pd` alone; `pd` and `pd$default$1` are public, and the pickled body selects the private
// getter itself (`pd[DefaultGetter 0]`). teq's runs print the .expected, scalac's. scalac typing b/ over teq's
// products of a/ (tests/modules' scalac-b, scalac-run, scalac-lean-b) fails with E173: "private method pd$default$1
// can only be accessed from object O". The whole build's classes also differ from the split build's (classes,
// subset, bundle). Found by reviewer #13 of the workers exports helper (push 8); the same on integration b602956824.
package pdgu

@main def run(): Unit =
  println(pdg.O.usePd)
