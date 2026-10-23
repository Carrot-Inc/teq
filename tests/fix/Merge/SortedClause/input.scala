/*
rules = [NoUnusedImports, "file:../../support/SortBlock.scala"]
*/
// An unused importee of a clause in a block another rule sorts: the clause collapses to its kept importee in round two.
package merge.clause
import merge.clause.Z.z
import merge.clause.O.{b, c}
object O { val b = 1; val c = 2 }
object Z { val z = 3 }
object Main { def run: Int = b + z }
