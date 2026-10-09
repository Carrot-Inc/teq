// expect: 8:8: error: expected an expression, found a blank line
// expect: 12:16: error: expected end of statement, found identifier
// expect: 16:11: error: expected end of statement, found literal
object Main:
  class Checked(val v: Int):
    infix def shouldBe(other: Int): Boolean = v == other
  def blank: Int =
    1 +

      2
  def ifOperand: Boolean =
    Checked(1) shouldBe
      if true then 1 else 2
  def leadingWord: Int =
    val r = 1
      max 2
    r
