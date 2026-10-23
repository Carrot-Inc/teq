// A token one rule replaces and another adds the same text to merges as scalafix's `PatchInternals.merge` makes it:
// the removal and both additions, `22`.
package merge.equal
object Numbers { val n = 22 }
