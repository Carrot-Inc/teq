/*
rules = [EmptyCollectionsUnified, DisableSyntax, NoUnusedImports]
DisableSyntax.noVars = true
lint.warning.includes = "DisableSyntax"
*/
// Round two over a program where another file stays as it was: there the semantic rules alone run again, the others'
// results being round one's, so `Other.scala` keeps its `var` reported and its anchor's suppression, no anchor unused.
package p
import scala.Nil
object Example { val empty: List[Int] = Nil }
