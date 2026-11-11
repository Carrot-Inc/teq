// Downstreams of a/ as tests/modules builds them. scalac 3.8.4 prints the .expected, and so does each of teq's runs.
// b/'s `bu` holds the expansion of a/'s transparent `tt`, which calls the inline `p` with its inline default. The
// whole build's pickle of `bu` is not scalac's: it wraps the expansion, the receiver `Fw.bx` and the default
// getter's call each in one INLINED more (TastyPrinter: INLINED(INLINED(...)), INLINED(SELECT bx), INLINED(SELECT
// p[DefaultGetter 0])), where scalac's has one INLINED around the expansion. The build over a/'s products writes
// scalac's shape. So the whole and split builds' classes of b/ and c/ differ (classes, subset, check-lean-whole).
// The Scala.js bundle built over the products outlines the getter's expansion (`p$default$1$o...()`,
// emit/outline.rs), where the whole bundle writes it in place (bundle). A plain inline method without a default
// passes every row. The same on integration a097de53d9 and on the tip of the workers exports helper's push 9.
// Reduced from tests/modules/exporttransparentdefault, whose four known.txt lines (classes, subset,
// check-lean-whole, bundle) this and its sibling pin replace, by the coordinator's ruling of 14:38 PST.
package ettc

@main def run(): Unit =
  println(ettb.B.bu)
  println(ett.tt + 4)
