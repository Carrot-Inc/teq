/*
rules = DisableSyntax
DisableSyntax.keywords = "var"
*/
// `keywords` is a list (`DisableSyntaxConfig`'s `Set[DisabledKeyword]`, a collection metaconfig reads from a list
// alone): a string there is a configuration error at its setting, never an empty set (the review of `a24540c28`).
object Config { var x = 1 }
