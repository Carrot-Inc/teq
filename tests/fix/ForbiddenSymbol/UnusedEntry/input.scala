/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = [
  "scala.collection.IterableOps.last",
  "scala.util.Random.nextGaussian",
  "fix.UnusedEntry.used"
]
*/
package fix

// The first two entries name members of the library that this program does not name: they are no dead entries.
object UnusedEntry:
  def used(x: Int): Int = x + 1 // assert: ForbiddenSymbol
  def user = used(1) // assert: ForbiddenSymbol
