/*
rules = [DisableSyntax]
DisableSyntax.noVars = true
lint.ignore.includes = "["
*/
// A lint filter's regex that does not compile is a configuration error at its setting, before any rule runs
// (`FilterMatcher.reader`'s fallible decoding), never a filter dropped (the review of `a24540c28`).
object Clean { val x = 1 }
