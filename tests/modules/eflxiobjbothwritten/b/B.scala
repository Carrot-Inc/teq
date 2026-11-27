// A local trait's default calling an object's inline def on the block's `x`, which the body reads too, every argument
// written: scalac prints 33. The readers' contract: tests/modules/efpure/a/A.scala.
package lxi_obj_both_written
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
