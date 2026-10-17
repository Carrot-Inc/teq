// A given whose type parameter the application's arguments leave open (`given [R]: Component[Unit, R]`
// for `Window.apply[A, R](content: A)(using Component[A, R])`): the search unifies the candidate's `R`
// with the application's (`Implicits.tryImplicit` adapts the candidate against the original `pt`), the
// application's interpolation leaves `R` open, held nonvariantly by `Window[Unit, R]`
// (`Inferencing.interpolateTypeVars`), and the expected type at the end of the chain fixes it, through
// one selection or two, beside the case class's own `apply` (an overload) as without a chain.
package givenopenvariablechain

trait Component[A, R]
object Component:
  given [R]: Component[Unit, R] = new Component[Unit, R] {}

final case class Window[A, R](content: A, c: Component[A, R], hasFocus: Boolean):
  def focus: Window[A, R] = copy(hasFocus = true)
  def withAnchor(n: Int): Window[A, R] = this

object Window:
  def apply[A, R](content: A)(using c: Component[A, R]): Window[A, R] = Window(content, c, false)

object Main:
  val w: Window[Unit, Unit] = Window(()).focus
  def make: Window[Unit, Int] = Window(content = ()).withAnchor(1)
  val two: Window[Unit, String] = Window(()).focus.withAnchor(1)
  val control: Window[Unit, Unit] = Window(())
  def main(args: Array[String]): Unit =
    println(w.hasFocus.toString + make.hasFocus.toString)
    println(two.hasFocus)
    println(control.hasFocus)
