// teq: --no-directives --werror
//> using options -Wconf:any:s
// Under --no-directives, which sbt's and Mill's plugins pass, no directive is read, as their scalac reads none: the
// warning stands and fails the build (scalac 3.8.4 given the file directly, docs/CLI.md).
object O { def f(): Unit = { 1; () } }
// expect: directive_none_read.scala:5:30: warning: A pure expression does nothing in statement position
