// Downstreams of a/ as tests/modules builds them. scalac
// 3.8.4 prints the .expected. b/'s `bt` holds the expansion of
// `Fw.tr()`, the export's forwarder of a transparent method with
// an inline default, whose getter is inline and not transparent
// (Desugar.scala:429), so scalac leaves its call for the inlining
// phase (`Inlines.needsInlining`) and selects it on the forwarder's
// owner, `Fw.tr$default$1` (`Namer.addForwarder`). teq's whole and
// split builds' classes of b/ and c/ are to be the same (classes,
// subset, check-lean-whole), and so their Scala.js bundles
// (bundle). Reduced from tests/modules/exporttransparentdefault,
// whose four known.txt lines (classes, subset,
// check-lean-whole, bundle) this and its sibling pin replace.
package etrc

@main def run(): Unit =
  println(etrb.B.bt)
  println(etr.Fw.tr() + 3)
