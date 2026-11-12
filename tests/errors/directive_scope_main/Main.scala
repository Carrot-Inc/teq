// A file's own `//> using target.scope main` makes it the main scope's whatever its name (CrossSources.baseReqs): the
// rule of Policy.test.scala holds for the main sources.
object Main { def f(): Unit = { 1; () } }
// expect: Main.scala:3:33: error: A pure expression does nothing in statement position
