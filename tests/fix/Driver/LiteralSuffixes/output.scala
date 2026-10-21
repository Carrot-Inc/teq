// A number literal's suffix is in its tree's span and in its ancestors': `replaceTree` replaces the whole literal, and
// a definition's text ends with it.
object LiteralSuffixes {
  val long = 2L // assert: Suffixes.val
  val float = 2.5f // assert: Suffixes.val
  val double = 3.0d // assert: Suffixes.val
  val upper = 0x10L /* assert: Suffixes.val
  ^^^^^^^^^^^^^^^^^
  val upper = 0x10L
  */
}
