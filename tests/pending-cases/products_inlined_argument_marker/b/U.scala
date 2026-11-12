// A downstream of a/ as tests/modules builds it. scalac 3.8.4
// prints the .expected. scalac's pickle of b/'s expansion of
// `q(10)` writes each use of the argument as an INLINED of
// no call around it, INLINED(INTconst 10) twice: the Inliner
// substitutes a parameter whose proxy is a singleton by
// `inlinedFromOutside(singleton(t))`, `Inlined(EmptyTree, Nil, ..)`
// (Inliner.scala:670-671, 703-716). teq's builds of b/,
// whole and over a/'s products, are to write it so, their
// classes of b/ the same (classes, subset, check-lean-whole).
package tbb

@main def run(): Unit =
  println(tba.G.q(10))
