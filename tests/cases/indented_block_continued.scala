// What follows an indented block continues it where no line separator stands before it: a selection, a type
// application, a `match`. The block continued is the one the line closes: a def's, a val's or a given's right
// side, an else branch at the `if`'s column (an `inline if`'s too) and the whole `if` at the def's, a yield's expression and the whole
// `for`, a try's body, a lambda's body, a named argument's value, a loop's body; a blank line or a comment before
// the line changes nothing, and an argument after the selection belongs to the selection. scalac prints the same
// lines.
case class Box(value: Option[Int]):
  def map(f: Int => Int): Box = Box(value.map(f))

class Thrower:
  def sel: String = throw new RuntimeException("selected")

extension (f: Int => String)
  def toUpperCase: Int => String = _ => "whole"

extension (u: Unit)
  def which: String = "outer-unit"
extension (s: String)
  def which: String = "inner-string"
  def shown: Unit = println(s)

def wrap(s: String): String = "X" + s
def take(x: Int): Int = x

inline def inlineElse(inline c: Boolean): Any =
  inline if c then
    1
  else
    2
  match
    case 1 => "one"
    case _ => "other"

object RightSides:
  def comprehension: Option[Int] =
    for
      x <- Box(Some(1))
    yield x + 1
  .value

  def statements =
    val x = "ab"
    x
  .toUpperCase

  val stored =
    val x = "cd"
    x
  .toUpperCase

  given text: String =
    val s = "ef"
    s
  .toUpperCase

  def blank: String =
    "gh"

  .toUpperCase

  def commented: String =
    val s = "ij"
    s
  /* a comment
   * continued */
  .toUpperCase

  def chained: String =
    List(1, 2)
  .map:
    x => x + 1
  .filter:
    x => x > 2
  .mkString(",")

  def braced: String =
    List(1, 2)
  .map {
    x => x + 1
  }
  .mkString(",")

  def matched(x: Int): String =
    x + 1

  match
    case 2 => "two"
    case _ => "other"

  def twoBlocks: String =
    val x =
      "ab"
  .which

  def innerWidth: String =
    val x =
      "kl"
    .toUpperCase
    "X" + x

  def farWidth: String =
    val x =
          "mn"
      .toUpperCase
    x

object Branches:
  def elseAlone(c: Boolean) = if c then
    "ab"
  else
    "cd"
  .toUpperCase

  def wholeIf(c: Boolean) =
    if c then "ab"
    else
      "cd"
  .toUpperCase

  def yieldExpression = for x <- List(1, 2) yield
    val s = "ab" + x
    s
  .toUpperCase

  def wholeFor =
    for x <- List(1, 2) yield
      val s = "ab" + x
      s
  .reverse

  def tried = try
    val b = new Thrower
    b
  .sel
  catch case e: RuntimeException => e.getMessage

  val lambda = (x: Int) =>
    val s = "op"
    s
  .toUpperCase

  val named = wrap(s =
    val x = "qr"
    x
  .toUpperCase)

  val applied = take(x =
    val id: [T] => T => T = [T] => (x: T) => x
    id
  [Int](42))

  val inParentheses = (
    if true then
      val id: [T] => T => T = [T] => (x: T) => x
      id
    [Int](43)
    else 0
  )

  def loops(): Unit =
    for x <- List(1, 2) do
      val s = "for" + x
      s
    .shown
    var n = 0
    while n < 2 do
      n += 1
      "while" + n
    .shown

@main def main(): Unit =
  import RightSides.{given, *}, Branches.*
  println((comprehension, statements, stored, summon[String], blank, commented))
  println((chained, braced, matched(1), matched(5), twoBlocks, innerWidth, farWidth))
  println((elseAlone(true), elseAlone(false), wholeIf(true), wholeIf(false)))
  println((yieldExpression, wholeFor, tried, lambda(0), named, applied, inParentheses))
  println((inlineElse(true), inlineElse(false)))
  loops()
