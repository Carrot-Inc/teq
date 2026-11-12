// teq: --werror
// The test scope's options stay out of the main sources' settings (CrossSources.sharedOptions, baseReqs):
// Policy.test.scala's by its name, test/T.scala's by its place, so the warning stands and fails the build.
object Main { def f(): Unit = { 1; () } }
// expect: Main.scala:4:33: warning: A pure expression does nothing in statement position
