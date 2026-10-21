/*
rules = NoUnusedImports
*/
// `macro_expansion` in a package block (the review of `a24540c28`): the expansion's site is the block, whose AST the
// parser lays out apart from the file's, and the import it may use stays, reported `unverified`.
package p {
import core.*
import p.* // assert: NoUnusedImports.unverified
enum Level derives En:
  case A, B
val e: En[Int] = null
}
