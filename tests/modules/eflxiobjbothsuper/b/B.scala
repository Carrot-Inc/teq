// A local trait's default calling an object's inline def on the block's `x`, which the body reads too, left by an
// override's super call: scalac prints 46. The readers' contract: tests/modules/efpure/a/A.scala.
package lxi_obj_both_super
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
