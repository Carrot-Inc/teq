// A local trait's second-clause default calling an object's inline def on the first clause and the block's `x`:
// scalac prints 34. The readers' contract: tests/modules/efpure/a/A.scala.
package lxi_obj_clause
object Main { def main(args: Array[String]): Unit = println(Lib.value(10)) }
