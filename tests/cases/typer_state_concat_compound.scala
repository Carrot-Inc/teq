//> using platform js
// An `if` whose condition is an operation over a plain inline call: scalac folds it
// once the call is expanded (an operation sees the constant, a bare `Boolean` call
// does not), which decides where the JVM renders a
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
  println((if M.yes && true then "" + new Loud else "") + tail())
  println((if M.maybe && true then "" + new Loud else "") + tail())
  println((if !M.yes then "" else "" + new Loud) + tail())
  println((if M.maybe then "" + new Loud else "") + tail())
