// expect: 10:3: error: expected end of statement, found 'else'
// After a condition in parentheses, a line further in that begins with a bracket goes on with the condition's
// line and opens no block: it alone is the branch, the statement on the line after it ends the `if`, and the
// `else` below has no `if` to go with. scalac: "end of toplevel definition expected but 'else' found" at the
// `else`.
def g(c: Boolean): Unit =
  if (c)
    (1 + 1).toString
    println("x")
  else println("y")
