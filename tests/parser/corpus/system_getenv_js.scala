//> using platform js
// interp-expected: js
// On JavaScript no environment variable is set, as Scala.js has it: `System.getenv` answers null
// (munit reads `NO_COLOR` so) and the whole environment is empty.
@main def run =
  println(System.getenv("NO_COLOR"))
  println(Option(System.getenv("HOME")).isDefined)
  println(System.getenv().isEmpty())
