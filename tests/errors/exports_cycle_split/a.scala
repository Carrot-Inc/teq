// expect: a.scala:8:10: error: cyclic export of B, which exports A
// The two objects of exports_cycle_alone in two files: given in either order (tests/split.sh
// runs both), the cycle is reported at the export of the object whose file comes first in the
// order of the files' identities.
package cyc

object A:
  export B.*
  def a: Int = 1
