// Downstreams of a/ as tests/modules builds them. scalac 3.8.4 prints the .expected, and so does each of teq's runs.
// teq's Scala.js bundles of the whole program and of c/ over a/'s and b/'s products differ in the name of the proxy
// of the inline `il`'s parameter in b/'s `all` (tests/modules' bundle): `y$proxy2` in the whole bundle, after the
// transparent `tr`'s, and `y$proxy1$1` in the other, which expands it from b/'s pickle and numbers it apart, as
// products_bundle_parent_lift_names' lifted arguments are. The same on integration a097de53d9. Found by the interim
// reviewer of the workers exports helper's push 10.
package pxc

@main def run(): Unit =
  println(pxb.U.all)
