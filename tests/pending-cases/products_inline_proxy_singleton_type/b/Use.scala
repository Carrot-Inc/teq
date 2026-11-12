// A downstream of a/ as tests/modules builds it. `IP.s(b)` is a
// transparent inline method whose body ascribes to the parameter's
// type member, `(b.get: b.T)`. scalac 3.8.4 prints the .expected.
// In b/'s pickle of `run` a type naming the expansion's binding
// proxy is met before that binding is written, where scalac
// writes a forward TERMREFdirect: teq's build over a/'s products
// is to write it too, b/'s classes and pickles the same between
// the builds (tests/modules' classes, subset, check-lean-whole).
package ipb

import ipa.*

@main def run(): Unit =
  val x: Int = IP.s(new IntBox2) + 1
  println(x)
