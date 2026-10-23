/*
rules = NoUnusedImports
*/
// An unused importee after a line comment that follows the comma before it: the comment stops `removeUpToFirstComma`
// (`ImportPatchOps.remove`), so scalafix 0.14.7 removes the importee without a comma and leaves one alone on its line,
// a text that does not parse; teq writes dotty's `checkImports` edit there, the importee with its comma.
package edits.comment_after_comma
object O { val a = 1; val b = 2; val c = 3 }
import O.{
  a, // the a
  b,
  c
}
object Main { def run: Int = a + c }
