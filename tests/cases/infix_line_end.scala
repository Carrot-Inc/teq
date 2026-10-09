// An alphanumeric operator ending its line takes the next line as its operand, a symbolic one
// too, and an annotation may follow a block's closing brace on the same line.
class Checked(val v: Int):
  infix def shouldBe(other: Int): Boolean = v == other
  infix def plus(other: Int): Checked = Checked(v + other)

object Main:
  def f(x: Int): Checked = Checked(x)

  def sameIndent: Boolean =
    val ok = f(3) shouldBe
    3
    ok

  def deeper: Boolean =
    f(4) shouldBe
      4

  def symbolic: Int =
    1 +
      2

  def commented: Int =
    val r = f(10) plus
      // the operand follows the comment
      5
    r.v

  def parenthesised: Boolean =
    f(1) shouldBe
      (if true then 1 else 2)

  def chained: Int =
    val r = f(1) plus
      2 plus
      3
    r.v

  def braced: Int = { 1 } : @unchecked

  def bracedNextLine: Int =
    { 2 }
    : @unchecked

  def ascribed: Int = { 3 } : Int

  def bodyMatch(x: Any): Int = {
    x match
      case i: Int => i
      case _ => 0
  } : @unchecked

  val (a, b) = (5: Any) match {
    case i: Int => (i, i + 1)
  } : @unchecked

  def main(args: Array[String]): Unit =
    println(sameIndent)
    println(deeper)
    println(symbolic)
    println(commented)
    println(parenthesised)
    println(chained)
    println(braced + bracedNextLine + ascribed)
    println(bodyMatch(7) + a + b)
