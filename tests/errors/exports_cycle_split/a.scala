// expect: a.scala:8:12: error: Conflicting definitions:
// The two objects of exports_cycle_alone in two files: given in either order (tests/split.sh
// runs both), the conflict is reported at the export of the object whose file comes first in
// the order of the files' identities, as scalac reports it.
package cyc

object A:
  export B.*
  def a: Int = 1
