// A local trait's second-clause default, the block's `x` read by the body, left on an anonymous class overriding the
// method: scalac prints 22. The readers' contract: tests/modules/efpure/a/A.scala.
package lt_local_body_override_anon_clause2
object Main:
  def main(args: Array[String]): Unit = println(Lib.value)
