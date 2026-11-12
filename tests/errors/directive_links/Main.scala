// A directory's links walked as scala-cli walks them (os-lib's walk follows no link; ElementsUtils keeps the entries
// os.isFile takes): W.scala, a link to a file, is that file, its -Werror read; lib, a link to a directory, brings
// nothing, neither P.scala's -Wconf:any:s nor its type error.
object Main { def f(): Unit = { 1; () } }
// expect: Main.scala:4:33: warning: A pure expression does nothing in statement position
// absent: P.scala
