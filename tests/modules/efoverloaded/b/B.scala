// An overloaded method's override default in a local trait, the downstream leaving it: scalac prints 22. The readers'
// contract: tests/modules/efpure/a/A.scala.
package esc
object Main { def main(args: Array[String]): Unit = println(Lib.make(10).m(2)()) }
