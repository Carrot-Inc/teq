// The variables of an overloaded call's chosen application that its interpolation left open, held
// nonvariantly by the result, stay open after the overload is resolved (dotty has no settlement of
// its own there: `Inferencing.interpolateTypeVars` of the trees around takes them): a factory
// overloaded or of a unique name alike; a bounded one (`R >: String`, no given), which the expected
// `Box[Any]` after `.focus` fixes above its bound; an argument `Window(())` of an overloaded method
// that its other argument chooses, or the expected type of its own chain; a secondary constructor's.
package givenopenvariableoverload

trait Component[A, R]
object Component:
  given [R]: Component[Unit, R] = new Component[Unit, R] {}

final case class Window[A, R](content: A, c: Component[A, R], hasFocus: Boolean):
  def focus: Window[A, R] = copy(hasFocus = true)

object Window:
  def apply[A, R](content: A)(using c: Component[A, R]): Window[A, R] = Window(content, c, false)
  def make[A, R](content: A)(using c: Component[A, R]): Window[A, R] = Window(content, c, false)

final case class Box[R](n: Int):
  def focus: Box[R] = this
object Build:
  def apply[R >: String](u: Unit): Box[R] = Box[R](0)
  def apply(i: Int): Int = i

object F:
  def f(w: Window[Unit, Int], n: Int): String = "int " + w.hasFocus
  def f(w: Window[Unit, String], s: String): String = "string " + w.hasFocus

final class Holder[R](val w: Window[Unit, R], val n: Int):
  def this(w: Window[Unit, R]) = this(w, 0)
  def this(s: String) = this(Window(()).asInstanceOf[Window[Unit, R]], s.length)
  def self: Holder[R] = this

object Main:
  val overloaded: Window[Unit, Unit] = Window(()).focus
  val unique: Window[Unit, Unit] = Window.make(()).focus
  val bounded: Box[Any] = Build(()).focus
  val h: Holder[Int] = new Holder(Window(())).self
  val k: Holder[String] = new Holder(Window(()).focus, 3).self
  def main(args: Array[String]): Unit =
    println(overloaded.hasFocus.toString + unique.hasFocus)
    println(bounded.n)
    println(F.f(Window(()), 1))
    println(F.f(Window(()).focus, "s"))
    println(h.n.toString + k.n + k.w.hasFocus)
