// A local trait's default calling an object's inline def on the block's `x`, which the body reads too, left on a
// local class: scalac prints 45. The readers' contract: tests/modules/efpure/a/A.scala.
package lxi_obj_both_left
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
