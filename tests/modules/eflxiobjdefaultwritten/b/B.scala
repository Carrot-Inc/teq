// A local trait's default calling an object's inline def on the block's `x`, every argument written: scalac prints 23.
// The readers' contract: tests/modules/efpure/a/A.scala.
package lxi_obj_default_written
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
