// A non-empty plain list on a trait's implicit clause, after the normalization's empty clause: its arguments are passed
// as written, by a parent and an anonymous class. Read at 3.6, where scalac gives no migration warning for it (the
// warning is tests/errors/trait_parent_plain_list_left_over's).
import scala.language.`3.6`
class A(val s: String)
class B(val s: String)
given A = A("ga")
given B = B("gb")
trait Ti(implicit a: A, b: B):
  def show: String = a.s + " " + b.s
class P1 extends Ti()(A("wa"), B("wb"))
object Main:
  def main(args: Array[String]): Unit =
    println(new P1().show)
    println((new Ti()(A("wa"), B("wb")) {}).show)
