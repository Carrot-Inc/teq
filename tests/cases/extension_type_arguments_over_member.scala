// A member that takes no type parameters, overloaded or not, given type arguments: dotty never applies it
// (Applications.typedTypeApply), and an extension of the name takes the call (Typer.tryInsertImplicitOnQualifier),
// as scalac calls it; without them the member is called.
class H:
  def w(x: Int): Unit = println("member " + x)
class Gate
extension (h: H)
  def w[A](a: A): Unit = println("extension " + a)
extension (g: Gate)
  def notify[A](a: A): Unit = println("notify extension " + a)
  def wait[A](a: A, b: Int): Unit = println("wait extension " + a + " " + b)
object Main:
  def main(args: Array[String]): Unit =
    new H().w[Int](2)
    new H().w(3)
    new Gate().notify[String]("n")
    new Gate().wait[String]("w", 1)
