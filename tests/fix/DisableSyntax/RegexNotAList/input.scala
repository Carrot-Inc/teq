/*
rules = DisableSyntax
DisableSyntax.regex = "TODO"
*/
// `regex` is a list (`DisableSyntaxConfig`'s `List[CustomMessage[...]]`): a string there is a configuration error at
// its setting, never an empty list (the review of `a24540c28`).
object Config { val todo = "TODO" }
