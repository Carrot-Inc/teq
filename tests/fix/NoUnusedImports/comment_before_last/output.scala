// An unused last importee after a line comment that follows the comma before it: scalafix 0.14.7 removes it without a
// comma and leaves `a,` with the braces gone, a text that does not parse; teq writes dotty's `checkImports` edit there,
// from the comma after the importee kept before it, the comment between with it.
package edits.comment_before_last
object O { val a = 1; val b = 2 }
import O.a
object Main { def run: Int = a }
