// A downstream of a/ as tests/modules builds it. `Named(..)`
// calls the apply of an upstream case class (no written
// companion). scalac 3.8.4 prints the .expected. scalac's pickle
// calls `Named.apply`; teq's whole build's is to call it too,
// b/'s pickle and classes the same in the whole build and the
// split one (tests/modules' classes, subset, check-lean-whole).
package zab

import tnba.*
import tnba.Keys.*

@main def run(): Unit =
  println(Named(Key("k"), SpaceId(4)).id)
