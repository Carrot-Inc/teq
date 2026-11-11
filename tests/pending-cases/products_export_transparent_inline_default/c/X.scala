// Downstreams of a/ as tests/modules builds them. scalac 3.8.4 prints the .expected, and so does each of teq's runs.
// b/'s `bt` holds the expansion of `Fw.tr()`, the export's forwarder of a transparent method with an inline default,
// whose getter is inline and not transparent (Desugar.scala:429), so scalac leaves its call for the inlining phase
// (`Inlines.needsInlining`) and selects it on the forwarder's owner, `Fw.tr$default$1` (`Namer.addForwarder`). The
// whole build expands the default at typing (TastyPrinter: TYPED(APPLY(SELECTin + (SELECT k), 3))); the build over
// a/'s products selects the member's getter on `bx` (SELECT tr[DefaultGetter 0] / SELECT bx), where scalac selects
// `Fw`'s. So the whole and split builds' classes of b/ and c/ differ (classes, subset, check-lean-whole). The
// Scala.js bundle built over the products outlines the getter's expansion (`tr$default$1$o...()`, emit/outline.rs),
// where the whole bundle writes it in place (bundle). The same on integration a097de53d9 and on the tip of the
// workers exports helper's push 9. Reduced from tests/modules/exporttransparentdefault, whose four known.txt lines
// (classes, subset, check-lean-whole, bundle) this and its sibling pin replace, by the coordinator's ruling of 14:38
// PST.
package etrc

@main def run(): Unit =
  println(etrb.B.bt)
  println(etr.Fw.tr() + 3)
