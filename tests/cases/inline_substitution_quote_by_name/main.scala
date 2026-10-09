// A by-name parameter in a stored body's quote: one of the inline method (`'expr`) is the
// argument's code where the quote runs, as scalac substitutes it; one a method of the quote
// defines (`force`) keeps the call of its thunk. A macro declared `Unit` whose splice's code is a
// quote (`${ '{ .. } }`, an `Expr[Unit]` where `Expr[Any]` is asked for) passes the definition's
// check. scalac prints the lines of the .expected file.
@main def run(): Unit =
  M.runIt
  println(M.twice)
  val x = 1
  M.show(x != 0)
  M.show(x == 0)
