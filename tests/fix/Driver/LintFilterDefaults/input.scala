/*
rules = [DisableSyntax]
DisableSyntax.noVars = true
DisableSyntax.noNulls = true
lint.warning = { excludes = "DisableSyntax.null" }
*/
// A filter object naming excludes alone includes everything else (`FilterMatcher.reader`'s default, `matchEverything`'s
// `.*`): `var`'s lint is a warning, `null`'s stays an error.
object Defaults {
  var x = 1 // assert: DisableSyntax.var
  val y: String = null // assert: DisableSyntax.null
}
