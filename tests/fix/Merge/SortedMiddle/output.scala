// An unused import in the middle of a block another rule sorts: its tokens and the line break before it lie in the
// sorting rule's removal, so round one writes the sorted text with it and round two removes it from there.
package merge.middle
import merge.middle.A.a
import merge.middle.C.c
object A { val a = 1 }
object C { val c = 2 }
object X { val x = 3 }
object Main { def run: Int = a + c }
