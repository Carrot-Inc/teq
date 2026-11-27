// A local trait declares the interface's method abstract with a default of its own, a local class implements it, the
// downstream leaves the default: scalac prints 22. The readers' contract: tests/modules/efpure/a/A.scala.
package esc
object Main { def main(args: Array[String]): Unit = println(Lib.make(10).m(2)()) }
