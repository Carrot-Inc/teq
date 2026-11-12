// teq: --no-directives --werror
//> using options -Wconf:any:s
// Under --no-directives no directive is to be read, as sbt's and Mill's scalac reads none: the warning
// stands and fails the build (scalac 3.8.4 given the file directly).
object O { def f(): Unit = { 1; () } }
// expect: directive_none_read.scala:5:30: warning: A pure expression does nothing in statement position
