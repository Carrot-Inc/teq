// A local class's override restating the default of the local trait it extends: scalac prints 23. The readers'
// contract: tests/modules/efpure/a/A.scala.
package esc
object Main { def main(args: Array[String]): Unit = println(Lib.make(10).m(2)()) }
