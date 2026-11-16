// jars: scala-library
// expect: 7:23: warning: trait DelayedInit in package scala is deprecated since 2.11.0: DelayedInit semantics can be surprising. Support for `App` will continue. See the release notes for more details: https://github.com/scala/scala/releases/tag/v2.11.0
// expect: 1 warning found, errors under --werror
// teq: --werror --deprecation
// A trait of the program extends the deprecated `DelayedInit` and nothing else of the build names a deprecated
// definition: the parent is warned of, also with scala-library's jar on the class path beside the std.
trait Journal extends DelayedInit { def delayedInit(x: => Unit): Unit = { println("journal"); x } }
object Main { def main(a: Array[String]): Unit = new Journal {}.delayedInit(println("ran")) }
