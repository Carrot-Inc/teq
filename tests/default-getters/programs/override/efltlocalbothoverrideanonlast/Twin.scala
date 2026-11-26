// Top-level twin of tests/modules/efltlocalbothoverrideanonlast: the local trait T at the top level, the block's `val x = 10` the holder's
// member; the anonymous class (it declares no default) kept where it was.
package lt_local_both_override_anon_last
object Holder:
  val x = 10
trait T { def m(a: Int, b: Int = Holder.x + 2): Int = a * 10 + b + Holder.x }
object Lib:
  def value: String =
    val r = (new T { override def m(a: Int, b: Int): Int = a * 100 + b }).m(1)
    r.toString
// A local trait's last parameter's default, the block's `x` read by the default and the body, left on an anonymous
// class overriding the method: scalac prints 32. The readers' contract: tests/modules/efpure/a/A.scala.
object Main:
  def main(args: Array[String]): Unit = println(Lib.value)
