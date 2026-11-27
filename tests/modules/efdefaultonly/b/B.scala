// A local trait's override, its second clause's default reading the first clause and the block's `x`: scalac prints
// 24. The readers' contract: tests/modules/efpure/a/A.scala.
package esc
object Main { def main(args: Array[String]): Unit = println(Lib.make(10).m(2)()) }
