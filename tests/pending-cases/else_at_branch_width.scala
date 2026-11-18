def f(c: Boolean): String =
  if c then
    "yes"
    else
    "no"
@main def main(): Unit = println(f(true) + f(false))
// An `else` at its branch's own column: the branch's indented block ends before it, as scalac's parser closes a
// region at a token that continues the statement, and the else's body on the next line opens a block of its own.
// scalac prints yesno.
