// A clause left with one rename keeps its braces (scalafix's `curlyBraceRemoves` drops them for a name or a wildcard
// alone), where dotty writes the selector's import.
package edits.rename_kept
object O { val x = 1; val y = 2 }
import O.{x as a}
object Main { def run: Int = a }
