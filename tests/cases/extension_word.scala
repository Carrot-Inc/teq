// The word `extension` begins an extension at a statement's start where a parameter clause follows it. A line that
// ends in an operand, a statement's body after its condition among them, ends its statement before one; a line
// that ends in an operator goes on with a call of a method of that name, and a blank line after it ends the
// statement; in backquotes the word is a name. The line after a self type's `=>` and after the `=>` of a lambda
// whose body is the rest of its block begins a statement. A parenthesis on a later line is no clause of an
// extension whose method stands on the header's line, and an extension declared in the block of a default
// argument of another's header leaves that header its methods. scalac prints the same lines.
import scala.language.postfixOps

object AfterOperand:
  def undefined: Int = ???
  extension (x: Int)
    def succ: Int = x + 1
  val negative = 2 + -1
  extension (x: String)
    def twice: String = x + x
  val no = !true
  extension (x: Boolean)
    def flip: Boolean = !x
  import scala.math.*
  extension (x: Long)
    def half: Long = x / 2
  val larger = negative max 5
  extension (x: Char)
    def up: Char = x.toUpper
  type Both = AfterOperand.type & Serializable
  extension (x: Short)
    def wide: Int = x.toInt
  var count = 0
  def touch: Unit = count += 1
  if (count == 0) touch
  extension (x: Byte)
    def next: Int = x + 1
  while (count < 3) touch
  extension (x: Float)
    def whole: Int = x.toInt

object AfterBlankLine:
  val xs = List(3, 1, 2)
  val count = xs length

  extension (x: Int)
    def doubled: Int = x * 2
  val total = xs size
  // a comment, then a blank line

  extension (x: String)
    def doubled: String = x + x

class SelfTyped {
  self =>
  extension (x: Int)
    def tripled: Int = x * 3
  def run: Int = 3.tripled
}

trait Mixed:
  this: SelfTyped =>
  extension (x: Int)
    def quadrupled: Int = x * 4
  def go: Int = 3.quadrupled

object InLambda:
  val square: Int => Int = {
    n =>
    extension (x: Int)
      def squared: Int = x * x
    n.squared
  }

object AfterOperator:
  def extension(x: Int): Int = x * 10
  val symbolic = 1 +
    extension(
      2)
      + 3
  val named = 1 max
    extension(
      2)
      + 3

object Quoted:
  def extension(x: Int)(y: Int): Int = x + y
  def block: Int =
    `extension`(1)(2)
  val value: Int = `extension`(3)(4)

object SameLine:
  extension (x: Int) def wrap: (Int, Int) = (x, x)
  val pair = (1,
    2)
  val after = 3

object InDefault:
  extension (x: Int)(using n: Int = { extension (y: Int) def inc: Int = y + 1; 1.inc })
    def result: Int = x + n
  extension (x: String)(using n: Int = {
    def value = 2
    value
  })
    def repeated: String = x * n

@main def main(): Unit =
  import AfterOperand.*
  println((1.succ, "ab".twice, true.flip, 8L.half, 'a'.up, negative, no, larger))
  println((2.toShort.wide, 2.toByte.next, 2.5f.whole, count))
  println((AfterOperator.symbolic, AfterOperator.named))
  println((Quoted.block, Quoted.value))
  import SameLine.*
  println((1.wrap, pair, after))
  import InDefault.*
  println((7.result, "ab".repeated))
  import AfterBlankLine.{count, doubled}
  println((count.doubled, AfterBlankLine.total, "ab".doubled, new SelfTyped().run, (new SelfTyped with Mixed).go, InLambda.square(4)))
