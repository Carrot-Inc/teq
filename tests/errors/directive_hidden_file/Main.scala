// teq: --werror
// A hidden file of a directory (`.policy.scala`) is no input of scala-cli's walk (ElementsUtils.singleFilesFromDirectory
// skips a name starting with `.`): neither its directive nor its code takes part, so the warning stands and fails the
// build under -Werror.
object Main { def f(): Unit = { 1; () } }
// expect: Main.scala:5:33: warning: A pure expression does nothing in statement position
// absent: .policy.scala
