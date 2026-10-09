//> using platform js
// A brace block is a simple expression that operators, a selection, `match` and an ascription
// continue, as an argument, a right-hand side, a branch or a lambda's body (scalac's
// `argumentExpr` and `blockExpr`). A block that ends in a concatenation continues it; one
// ascribed or annotated ends it.
class Loud(s: String):
  override def toString: String =
    println("  render " + s)
    s

def effect(): String =
  println("  effect")
  "!"

def product: Int = { 2 } * 3 + 1
def selected: Int = { "abc" }.length * 2
def precedence: Int = { 2 } + { 3 } * 4

def minus(a: Int, b: Int): Int = a - b

class Box(val n: Int):
  def +=(b: Box): Box = new Box(n - b.n)

@main def main(): Unit =
  val x = 1
  val y = "!"
  println({ val h = "h:"; h + x } + y)
  println({ 2 } * 3 + 1)
  println(List(product, selected, precedence))
  println({ 5 } match { case 5 => "five"; case _ => "other" })
  println({ 1 } :: { 2 } :: Nil)
  println({ 1 } == 1)
  println({ 3 } max 4)
  println(List(1, 2).map(n => { n } * 10))
  println(if false then { 1 } else { 2 } + 3)
  println(if true then { 1 } else { 2 } + 3)
  println(minus(b = { 4 } - 1, a = 2))
  println((new Box(1) += { new Box(2) } += new Box(3)).n)
  println(List(1, 2).map({ 10 } + _))
  val plus10: Int => Int = { 10 } + _
  println(plus10(5))
  val l = new Loud("x")
  println("block then effect"); println({ println("  stat"); "" + l } + effect())
  println("annotated block then effect"); println(({ println("  stat"); "" + l }: @unchecked) + effect())
  println("ascribed block then effect"); println(({ println("  stat"); "" + l }: String) + effect())
  println("block then a longer chain"); println({ "" + l } + new Loud("y") + effect())
