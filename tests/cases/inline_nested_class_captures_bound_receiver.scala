// An inline call's receiver bound before its body is read by a class that a nested expansion in that body makes:
// the nested call on `this`, or on `this` passed as an argument, reads the outer receiver, and the class the
// nested body makes, with the lambda in its method, holds it. Two levels and three, the receiver an unstable
// expression or a call.
trait Shower:
  def show(): String
trait Deriv:
  def name: String
  inline def leaf(): Shower = new Shower { def show(): String = { val f = (i: Int) => "leaf " + name + i; f(1) } }
  inline def derived: Shower = new Shower { def show(): String = leaf().show() }
  inline def deeper: Shower = new Shower { def show(): String = new Shower { def show(): String = leaf().show() }.show() }
  inline def passed: Shower = new Shower { def show(): String = Deriv.leafOf(Deriv.this).show() }
object Deriv:
  inline def leafOf(d: Deriv): Shower = new Shower { def show(): String = { val f = (i: Int) => "leafOf " + d.name + i; f(1) } }
class Impl(val name: String) extends Deriv
object Loud extends Deriv:
  println("loud")
  val name: String = "L"
def mk(n: String): Impl = { println("mk " + n); Impl(n) }
@main def run(): Unit =
  val flag = mk("flag").name.nonEmpty
  println((if flag then Loud else Loud).derived.show())
  println(mk("a").derived.show())
  println(mk("b").deeper.show())
  println(mk("c").passed.show())
  val v = Impl("v")
  println(v.derived.show())
