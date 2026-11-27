// A local trait's default calling an object's inline def on the block's `x`, left on an anonymous class: scalac
// prints 31. The readers' contract: tests/modules/efpure/a/A.scala.
package lxi_obj_anon
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
