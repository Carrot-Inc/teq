/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// Positions in UTF-16 units past text of more than one byte a character (é, ü), and of two units (😀), as scalameta
// 4.17.4's and the API's.
object Unicodé:
  val marker = 0 // assert: DumpLayout.layout
  val grüße = "😀 und 世界"
  def café(x: Int): Int = x + 1
  val after = café(1)
