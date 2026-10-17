// The open parameter is kept open only as far as the trees around the application: a `val` with no
// declared type makes it fully defined (`Window[Unit, Nothing]`), so its `.focus` is no
// `Window[Unit, Unit]`; a given that fixes it (`Fixed[Unit, String]`) or bounds it
// (`R <: CharSequence`) decides it before the chain's expected type is met; content of another type
// stays refused; an overload with `Window(())` as an argument refuses as scalac does where none of
// its alternatives takes the other argument, or where two take it alike, and so does a secondary
// constructor.
import scala.annotation.targetName

trait Component[A, R]
object Component:
  given [R]: Component[Unit, R] = new Component[Unit, R] {}
trait Fixed[A, R]
object Fixed:
  given Fixed[Unit, String] = new Fixed[Unit, String] {}
trait Bounded[A, R]
object Bounded:
  given [R <: CharSequence]: Bounded[Unit, R] = new Bounded[Unit, R] {}

final case class Window[A, R](content: A, hasFocus: Boolean):
  def focus: Window[A, R] = copy(hasFocus = true)
object Window:
  def apply[A, R](content: A)(using Component[A, R]): Window[A, R] = Window(content, false)
  def fixed[A, R](content: A)(using Fixed[A, R]): Window[A, R] = Window(content, false)
  def fixed(n: Int): Int = n
  def bounded[A, R](content: A)(using Bounded[A, R]): Window[A, R] = Window(content, false)
  def bounded(n: Int): Int = n

object F:
  def f(w: Window[Unit, Int], n: Int): Int = 1
  def f(w: Window[Unit, String], s: String): Int = 2
  def g(w: Window[Unit, Int]): Int = 1
  @targetName("gString") def g(w: Window[Unit, String]): Int = 2

final class Holder[R](val w: Window[Unit, R], val n: Int):
  def this(w: Window[Unit, R]) = this(w, 0)

object Main:
  val saved = Window(())
  val boundary: Window[Unit, Unit] = saved.focus
  val fixed: Window[Unit, Unit] = Window.fixed(()).focus
  val bounded: Window[Unit, Unit] = Window.bounded(()).focus
  val content: Window[Int, Unit] = Window(()).focus
  val none = F.f(Window(()), true)
  val ambiguous = F.g(Window(()))
  val holder = new Holder(Window(()), "x")
// expect: 40:38: error: type mismatch: found Window[Unit, Nothing], required Window[Unit, Unit]
// expect: 41:35: error: type mismatch: found Window[Unit, String], required Window[Unit, Unit]
// expect: 42:37: error: type mismatch: found Window[Unit, CharSequence], required Window[Unit, Unit]
// expect: 43:36: error: type mismatch: found Window[Unit, Nothing], required Window[Int, Unit]
// expect: 44:14: error: None of the overloaded alternatives of method f in object F with types
// expect: match arguments (Window[Unit, Nothing], true)
// expect: 45:19: error: Ambiguous overload. The overloaded alternatives of method g in object F with types
// expect: both match arguments (Window[Unit, Nothing])
// absent: None of the overloaded alternatives of method g
// expect: 46:39: error: type mismatch: found "x", required Int
