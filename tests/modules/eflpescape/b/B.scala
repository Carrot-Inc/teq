// A local trait's override's default reading nothing of the block, the class returned as the interface and called by
// the module's Caller: scalac prints 20. The readers' contract: tests/modules/efpure/a/A.scala.
package lp_escape
object Main { def main(args: Array[String]): Unit = println(Caller.call(Lib.make(10))) }
