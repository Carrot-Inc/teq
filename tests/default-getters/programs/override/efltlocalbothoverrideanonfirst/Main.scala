// A local trait's first parameter's default, the block's `x` read by the default and the body, left on an anonymous
// class overriding the method: scalac prints 141. The readers' contract: tests/modules/efpure/a/A.scala.
package lt_local_both_override_anon_first
object Main:
  def main(args: Array[String]): Unit = println(Lib.value)
