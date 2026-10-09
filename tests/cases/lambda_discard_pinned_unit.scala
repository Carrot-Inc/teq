// A lambda's value is discarded for a type parameter bounded above by `Unit` where the call is
// expected to be a `Unit => Unit` for an `A => Unit` or a `Box[Unit]` for a `Box[A]`.
final case class Box[A](a: A)
def f[A <: Unit](g: () => A): A => Unit = a => println(s"f $a")
def k[A <: Unit](g: () => A): Box[A] = Box(g())

@main def main(): Unit =
  val x: Unit => Unit = f(() => 1)
  x(())
  val y: Box[Unit] = k(() => 2)
  println(y)
