/*
rules = ["file:../../support/Suffixes.scala"]
*/
// A number literal's suffix is in its tree's span and in its ancestors': `replaceTree` replaces the whole literal, and
// a definition's text ends with it.
object LiteralSuffixes {
  val long = 1L // assert: Suffixes.val
  val float = 1.5f // assert: Suffixes.val
  val double = 2.0d // assert: Suffixes.val
  val upper = 0x10L /* assert: Suffixes.val
  ^^^^^^^^^^^^^^^^^
  val upper = 0x10L
  */
}
