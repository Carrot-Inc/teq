// The directory's files in scala-cli's order, their names compared by UTF-16 code units (ElementsUtils
// .singleFilesFromDirectory): U+10000's file, `any:s`, before U+E000's, `any:e`, whose rule decides; by
// UTF-8 bytes the order is the other way round.
object Main { def f(): Unit = { 1; () } }
// expect: Main.scala:4:33: error: A pure expression does nothing in statement position
