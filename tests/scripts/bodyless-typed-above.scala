// Ledger's constructor is empty, but above it Tally's construction prints, which the interpreter does not run
// from a Java level.
class Sheet extends jlib.Ledger

@main def run(): Unit =
  new Sheet()
  println("done")
