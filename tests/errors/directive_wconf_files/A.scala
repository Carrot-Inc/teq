//> using options -Wconf:any:s
// The rules of every file's directive, in the files' order (scala-cli's for a directory, its files by path), the
// rightmost deciding: B.scala's `any:e` over A.scala's `any:s` (docs/CLI.md).
object A { def f(): Unit = { 1; () } }
// expect: A.scala:4:30: error: A pure expression does nothing in statement position
