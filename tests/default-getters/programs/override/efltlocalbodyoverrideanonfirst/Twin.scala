// Top-level twin of tests/modules/efltlocalbodyoverrideanonfirst: the local trait T at the top level, the block's `val x = 10` the holder's
// member; the anonymous class (it declares no default) kept where it was.
package lt_local_body_override_anon_first
object Holder:
  val x = 10
trait T { def m(a: Int = 3, b: Int): Int = a * 10 + b + Holder.x }
object Lib:
  def value: String =
    val r = (new T { override def m(a: Int, b: Int): Int = a * 100 + b }).m(b = 1)
    r.toString
// A local trait's first parameter's default, the block's `x` read by the body, left on an anonymous class overriding
// the method: scalac prints 41. The readers' contract: tests/modules/efpure/a/A.scala.
object Main:
  def main(args: Array[String]): Unit = println(Lib.value)
