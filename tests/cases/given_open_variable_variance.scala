// Without an expected type the open parameter is instantiated where the definition's type is made
// fully defined (`Inferencing.isFullyDefined`, `instDecision` by its variance and bounds): a covariant
// `Source[R]` is `Source[Nothing]`, a contravariant `Sink[R]` is `Sink[Any]`, and the invariant
// `Window[Unit, R]`, its `R` unbounded, is minimised to `Window[Unit, Nothing]`.
package givenopenvariablevariance

trait C[R]
object C:
  given [R]: C[R] = new C[R] {}
final class Source[+R]:
  def self: Source[R] = this
final class Sink[-R]:
  def self: Sink[R] = this
object Build:
  def source[R](u: Unit)(using C[R]): Source[R] = new Source[R]
  def source(n: Int): Int = n
  def sink[R](u: Unit)(using C[R]): Sink[R] = new Sink[R]
  def sink(n: Int): Int = n

trait Component[A, R]
object Component:
  given [R]: Component[Unit, R] = new Component[Unit, R] {}
final case class Window[A, R](content: A, c: Component[A, R], hasFocus: Boolean):
  def focus: Window[A, R] = copy(hasFocus = true)
object Window:
  def apply[A, R](content: A)(using c: Component[A, R]): Window[A, R] = Window(content, c, false)

object Main:
  val so = Build.source(()).self
  val si = Build.sink(()).self
  val bottom: Source[Nothing] = so
  val top: Sink[Any] = si
  val w = Window(()).focus
  val check: Window[Unit, Nothing] = w
  def main(args: Array[String]): Unit = println("ok " + w.hasFocus)
