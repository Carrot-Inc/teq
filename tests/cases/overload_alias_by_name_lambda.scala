// Overloads that differ in a by-name parameter of an alias type and a function parameter (zio's
// `onInterrupt(cleanup: => URIO[R1, Any])` beside `onInterrupt(cleanup: Set[FiberId] => ..)`): a
// function literal goes to the function, the alias being dealiased before the literal is
// matched against it.

class Trace
object Trace:
  implicit val t: Trace = Trace()
type URIO[-R, +A] = Box[R, Nothing, A]
class Box[-R, +E, +A](val a: A):
  final def on[R1 <: R](cleanup: => URIO[R1, Any])(implicit trace: Trace): Box[R1, E, A] = this
  final def on[R1 <: R](cleanup: Set[Int] => URIO[R1, Any])(implicit trace: Trace): Box[R1, E, A] = this
object Box:
  val unit: Box[Any, Nothing, Unit] = Box(())
object Main:
  def main(args: Array[String]): Unit =
    println(Box[Any, Throwable, Int](1).on(_ => Box.unit).a)
    println(Box[Any, Throwable, Int](1).on(Box.unit).a)
