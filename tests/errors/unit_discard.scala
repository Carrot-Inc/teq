// expect: 6:17: warning: Discarded non-Unit value of type Int. Add `: Unit` to discard silently.
// expect: 9:3: warning: A pure expression does nothing in statement position
// expect: 13:3: warning: Discarded non-Unit value of type String. Add `: Unit` to discard silently.
// expect: 17:17: warning: Discarded non-Unit value of type String. Add `: Unit` to discard silently.
// expect: 20:18: error: type mismatch: found String, required Int
def f(): Unit = 5

def g: Int =
  1
  2

def h(): Unit =
  "s"

@main def run(): Unit =
  f()
  val u: Unit = "s"
  println(u)
  println(g)
  val bad: Int = "x"
