//> using target.scope main
// scala-cli 1.17.1 refuses the experimental `//> using target.scope` without --power (its directive's error); teq reads
// it as --power does, a file's scope its own (docs/CLI.md).
@main def run(): Unit = println(1)
