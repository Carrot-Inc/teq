// jars: scala-library
// expect: 6:7: error: class Ledger needs to be abstract, since def delayedInit(x: => Unit): Unit in trait DelayedInit is not defined
// expect: 6:22: warning: trait DelayedInit in package scala is deprecated since 2.11.0: DelayedInit semantics can be surprising. Support for `App` will continue. See the release notes for more details: https://github.com/scala/scala/releases/tag/v2.11.0
// teq: --deprecation
// A class that extends the deprecated `DelayedInit` without its method: the parent's warning beside the error.
class Ledger extends DelayedInit
object Main { def main(a: Array[String]): Unit = println(new Ledger) }
