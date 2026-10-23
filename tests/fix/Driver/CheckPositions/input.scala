/*
rules = [NoUnusedImports, DisableSyntax]
DisableSyntax.noVars = true
*/
// `--check` prints round one's lints at the file's own lines, as scalafix's `--check` does, and a rewrite those of its
// last round at the lines of the file it writes: the `var` below is line 10 here and line 9 once the unused import
// above it goes.
package driver.checkpositions
import scala.collection.mutable.Buffer
object O { var x = 1 } // assert: DisableSyntax.var
