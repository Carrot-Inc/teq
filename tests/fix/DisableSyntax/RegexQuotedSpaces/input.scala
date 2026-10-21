/*
rules = DisableSyntax
DisableSyntax.regex = [{ id = spaced, pattern = " v[a]r " }]
*/
// A quoted setting keeps its characters as written, its spaces among them (the review of `86884ea36`): the pattern
// matches a word between spaces, not a word's start.
object Input {
  val s = "variable"
  val t = "a var b" // assert: DisableSyntax.spaced
}
