//> using target.scope main
// scala-cli 1.17.1 refuses the experimental `//> using target.scope`
// without --power (its directive's error).
@main def run(): Unit = println(1)
