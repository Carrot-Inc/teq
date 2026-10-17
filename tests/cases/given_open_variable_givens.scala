// The given's open parameter kept through the chain whatever the given's shape: two type parameters,
// both unified with the application's (`given [A, R]: Pair[A, R]`, `A` fixed by the content); a given
// in the content type's companion (`Content.given [R]: Component[Content, R]`); a given whose own
// using clauses take the open parameter (`given [R](using Marker[R], Flag[R])`), the nested searches
// leaving it open as the outer one does. Each factory is overloaded, as `Window.apply` is beside the
// case class's own.
package givenopenvariablegivens

trait Pair[A, R]
object Pair:
  given [A, R]: Pair[A, R] = new Pair[A, R] {}

trait Component[A, R]
final class Content
object Content:
  given [R]: Component[Content, R] = new Component[Content, R] {}

trait Marker[R]
object Marker:
  given [R]: Marker[R] = new Marker[R] {}
trait Flag[R]
object Flag:
  given [R]: Flag[R] = new Flag[R] {}
trait Nested[A, R]
object Nested:
  given [R](using Marker[R], Flag[R]): Nested[Unit, R] = new Nested[Unit, R] {}

final case class Window[A, R](content: A, hasFocus: Boolean):
  def focus: Window[A, R] = copy(hasFocus = true)

object Window:
  def paired[A, R](content: A)(using Pair[A, R]): Window[A, R] = Window(content, false)
  def paired(n: Int): Int = n
  def apply[A, R](content: A)(using Component[A, R]): Window[A, R] = Window(content, false)
  def nested[A, R](content: A)(using Nested[A, R]): Window[A, R] = Window(content, false)
  def nested(n: Int): Int = n

object Main:
  val p: Window[String, Int] = Window.paired("x").focus
  val c: Window[Content, Int] = Window(new Content).focus
  val n: Window[Unit, Unit] = Window.nested(()).focus
  def main(args: Array[String]): Unit =
    println(p.content + " " + p.hasFocus)
    println(c.hasFocus)
    println(n.hasFocus)
