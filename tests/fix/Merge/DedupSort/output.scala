// A token one rule removes and another adds to is the addition (scalafix's `PatchInternals.merge`): the first import,
// written again later, removed by one rule, and the sorted block added left of its first token by the other.
package merge.dedup
import merge.dedup.A.a
import merge.dedup.B.b
object A { val a = 1 }
object B { val b = 2 }
object Main { def run: Int = a + b }
