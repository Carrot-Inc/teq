// A brace block is as wide as its first line: a right side whose value stands on that line opens no block of its
// own there, and a `for` or an `if` begun on the brace's line takes the next lines with its `do` or `else` further
// left. Braces whose first line break falls inside parentheses are as wide as the region around them, so a later
// line of theirs that begins with a parenthesis goes on with the line before it. scalac prints the same lines.
class F { def apply(x: Int): String = "applied"; override def toString = "value" }

object TwoMembers { val x =
    1
    val y = 2
}

object ForOnBraceLine {
  var log = ""
  def run(): String = {
    List(1, 2).foreach { x => for
        y <- List(x, x)
        z <- List(y * 2)
      do log += z }
    log
  }
}

object IfOnBraceLine {
  def run(): List[String] = List(1, 9).map { x => if x > 5 then
      "big"
    else "small" }
}

object WidthFromOutside { def f(a: Int,
    b: Int) = a
  val g = new F
  val h = g
  (41)
}

@main def main(): Unit =
  println(TwoMembers.x + TwoMembers.y)
  println(ForOnBraceLine.run())
  println(IfOnBraceLine.run())
  println(WidthFromOutside.h)
