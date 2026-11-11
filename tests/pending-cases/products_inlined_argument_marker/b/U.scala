// A downstream of a/ as tests/modules builds it. scalac 3.8.4 prints the .expected, and so does each of teq's runs.
// scalac's pickle of b/'s expansion of `q(10)` writes each use of the argument as an INLINED of no call around it,
// INLINED(INTconst 10) twice: the Inliner substitutes a parameter whose proxy is a singleton by
// `inlinedFromOutside(singleton(t))`, `Inlined(EmptyTree, Nil, ..)` (Inliner.scala:670-671, 703-716). teq's build of
// b/ over a/'s products writes the bare `INTconst 10` twice. Its whole build writes the markers, with one INLINED
// more around the expansion (products_transparent_expansion_inlined_layers). So the whole and split builds' classes
// of b/ differ (classes, subset, check-lean-whole). The same on integration a097de53d9. Found beside
// overloadexpand's second cause: with the INLINED nodes set aside, tests/modules/overloadexpand's whole and split
// pickles of b/ are identical, so its classes, subset and check-lean-whole rows are this and the sibling pin's.
package tbb

@main def run(): Unit =
  println(tba.G.q(10))
