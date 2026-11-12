// Downstreams of a/ as tests/modules builds
// them. scalac 3.8.4 prints the .expected. teq's
// Scala.js bundles of the whole program and of
// c/ over a/'s and b/'s products are to agree
// in the name of the proxy of the inline `il`'s
// parameter in b/'s `all` (tests/modules' bundle).
package pxc

@main def run(): Unit =
  println(pxb.U.all)
