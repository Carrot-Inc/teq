// A downstream of a/ as tests/modules builds it. `Named(..)` calls the apply of an upstream case class (no written
// companion). scalac 3.8.4 prints the .expected, and so does each of teq's runs. scalac's pickle, and teq's over a/'s
// products, call `Named.apply`; teq's whole build pickles `new Named(..)` (SELECTin <init>), so b/'s pickle and
// classes differ between the whole build and the split one (tests/modules' classes, subset, check-lean-whole). Cause
// not located. The same on integration b602956824; visible in tests/modules/typenamedbinding once the SELECTtpt port
// lets its products build.
package zab

import tnba.*
import tnba.Keys.*

@main def run(): Unit =
  println(Named(Key("k"), SpaceId(4)).id)
