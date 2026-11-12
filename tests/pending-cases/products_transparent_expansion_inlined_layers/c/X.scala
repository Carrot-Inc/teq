// Downstreams of a/ as tests/modules builds them. scalac
// 3.8.4 prints the .expected. b/'s `bu` holds the expansion
// of a/'s transparent `tt`, which calls the inline `p`
// with its inline default. scalac's pickle of `bu` has
// one INLINED around the expansion; teq's whole build's is
// to be scalac's, the whole and split builds' classes of
// b/ and c/ the same (classes, subset, check-lean-whole),
// and so their Scala.js bundles (bundle). Reduced from
// tests/modules/exporttransparentdefault, whose four
// known.txt lines (classes, subset, check-lean-whole,
// bundle) this and its sibling pin replace.
package ettc

@main def run(): Unit =
  println(ettb.B.bu)
  println(ett.tt + 4)
