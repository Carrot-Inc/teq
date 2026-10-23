/*
rules = [NoUnusedImports, "file:../../support/SortBlock.scala"]
*/
// An unused last import of a block another rule sorts: the line break before it lies inside the sorting rule's
// removal and the one after it outside, so its line loses the one before it (scalafix's); round two removes it from
// the sorted text, where it is first.
package merge.last
import merge.last.X.x
import merge.last.C.c
import merge.last.A.a
object A { val a = 1 }
object C { val c = 2 }
object X { val x = 3 }
object Main { def run: Int = c + x }
