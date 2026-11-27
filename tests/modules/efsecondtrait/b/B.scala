// A local trait's override's default reached through a second local trait: scalac prints 22. The readers' contract:
// tests/modules/efpure/a/A.scala.
package esc
object Main { def main(args: Array[String]): Unit = println(Lib.make(10).m(2)()) }
