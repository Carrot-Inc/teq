//> using platform js
// An `if` on a plain inline call: scalac's typer folds the condition by its type alone, a constant
// type (`yes: true`) and not a `Boolean` (`maybe: Boolean`), which decides where the JVM renders a
// concatenation's operand.
class Loud:
  override def toString: String =
    println("render")
    "x"

object M:
  inline def yes: true = true
  inline def maybe: Boolean = true

def tail(): String =
  println("tail")
  "!"

@main def main(): Unit =
  println((if M.yes then "" + new Loud else "") + tail())
  println((if M.maybe then "" + new Loud else "") + tail())
