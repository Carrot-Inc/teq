// A local trait's override's default left by the downstream, the block's `x` read by the body: scalac prints 37. The
// readers' contract: tests/modules/efpure/a/A.scala.
package lx_escape_body_left
object Caller { def call(t: API): Int = { t.m(2) } }
object Main { def main(args: Array[String]): Unit = println(Caller.call(Lib.make(10))) }
