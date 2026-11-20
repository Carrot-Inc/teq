// A package object in indented form beside files of its package in module b: its wrapper and its package's reference
// at its `package` keyword (`objectDef(start, ..)`), in module a's build alone (parsed on one thread) and in the whole
// build, which b's generated sibling makes large enough to be parsed in parallel (`frontend::parse_files`): a place the
// parser records is a function of the file, whichever thread read it (the gate of pos-2's application chain).
package outerpk

import scala.math.Ordering

package object innerpkg:

  implicit class Mult(inst: Int):
    def times(that: Int): Int = Ordering.Int.max(inst * that, 0)

  def unit: Int = 1
