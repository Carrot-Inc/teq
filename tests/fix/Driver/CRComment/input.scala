/*
rules = [DisableSyntax]
DisableSyntax.noVars = true
lint.warning.includes = "DisableSyntax"
*/
// A line comment ends at a CR as at an LF (dotty's `Scanners.skipLine`): `Lone.scala`'s lines end in a CR alone, so
// its comment ends before its object, whose `var` is reported.
object CRComment
