/*
rules = ["file:../../support/LiteralUnits.scala"]
*/
// The literals' values as UTF-16 units, as scalameta 4.17.4 reads them (the review of 2f7c9255a): a string's value a
// `String`, a lone surrogate's unit kept in a string, an interpolation part and a char, a pair one character.
object LiteralUnits {
  val plain = "hello" // assert: LiteralUnits.string
  val lone = "\uD800" // assert: LiteralUnits.string
  val pair = "😀" // assert: LiteralUnits.string
  val part = s"a\uD800b" // assert: LiteralUnits.string
  val raw = raw"\uD800" // assert: LiteralUnits.string
  val c = '\uD800' // assert: LiteralUnits.char
  val d = 'a' // assert: LiteralUnits.char
}
