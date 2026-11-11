// A downstream of a/ as tests/modules builds it. `IP.s(b)` is a transparent inline method whose body ascribes to the
// parameter's type member, `(b.get: b.T)`. scalac 3.8.4 prints the .expected, and so does each of teq's runs. Built
// over a/'s products, b/'s pickle of `run` is withheld ("elided: unstated the singleton type of the local b$proxy1",
// src/tasty/write/pickle.rs) where the whole build writes it: a type naming the expansion's binding proxy is met before
// that binding is written, where scalac and teq's whole build write a forward TERMREFdirect. So b/'s classes and
// pickles differ between the builds (tests/modules' classes, subset, check-lean-whole). `(b: b.type)` meets the same
// before the SELECTtpt port (reviewer #14's rv14tsel-singleton on tip34); the type selection reaches it since
// 52720e53bf. Found by reviewer #14 of the workers exports helper (push 9).
package ipb

import ipa.*

@main def run(): Unit =
  val x: Int = IP.s(new IntBox2) + 1
  println(x)
