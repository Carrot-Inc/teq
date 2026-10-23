/*
rules = [NoUnusedImports, "file:../../support/SortBlock.scala"]
*/
// An unused first import of a block another rule sorts, after a blank line: the line break before it lies outside
// the sorting rule's removal and the one after it inside, so its line loses the one after it and the sorted text
// stays on a line of its own; round two removes it from there.
package merge.firstblank

import merge.firstblank.X.x
import merge.firstblank.A.a
object A { val a = 1 }
object X { val x = 3 }
object Main { def run: Int = a }
