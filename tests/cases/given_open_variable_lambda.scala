// The real shape: two written `apply`s, one taking a function of the open `R` in a second list, and a
// chain ending in a lambda over `R` (`withBackground(_ => 1)`), the expected type of the definition
// fixing `R` through the chain.
package givenopenvariablelambda

trait Component[A, R]
object Component:
  given [R]: Component[Unit, R] = new Component[Unit, R] {}

final case class Window[A, R](content: A, c: Component[A, R], hasFocus: Boolean):
  def focus: Window[A, R] = copy(hasFocus = true)
  def withAnchor(n: Int): Window[A, R] = this
  def withBackground(f: R => Int): Window[A, R] = this

object Window:
  def apply[A, R](content: A)(using c: Component[A, R]): Window[A, R] = Window(content, c, false)
  def apply[A, R](content: A)(f: R => Int)(using c: Component[A, R]): Window[A, R] = Window(content, c, false)

object Main:
  val w: Window[Unit, Unit] = Window(()).focus
  def make: Window[Unit, Int] = Window(content = ()).withAnchor(1).withBackground(n => n + 1)
  def curried: Window[Unit, String] = Window(())((s: String) => s.length).focus
  def main(args: Array[String]): Unit =
    println(w.hasFocus.toString + make.hasFocus)
    println(curried.hasFocus)
