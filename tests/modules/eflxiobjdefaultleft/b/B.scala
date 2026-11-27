// A local trait's default calling an object's inline def on the block's `x`, left on a local class: scalac prints 35.
// The readers' contract: tests/modules/efpure/a/A.scala.
package lxi_obj_default_left
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
