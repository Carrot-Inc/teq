/*
rules = DisableSyntax
disableSyntax.noVar = true
*/
// A setting of the section a rule reads, its alias `disableSyntax` too, that its decoding does not read is an error
// at its place (metaconfig's unknown field; the review of `86884ea36`), as `DisableSyntax.noVar` is.
object Input { var x = 1 }
