// A lambda whose result goes to a variable bounded above by `Unit` (a mail client's request
// helper `req[A](run: Client => A): Box[A]` for a `Box[Unit]`): the variable is `Unit` and the
// body's value is discarded, as scalac adapts the body again once the variable is instantiated.
final case class Box[+A](a: A)
object Reqs:
  def req[A](op: String)(run: String => A): Box[A] = Box(run(op))
  def del(): Box[Unit] = req("x"): s =>
    s.isEmpty
  def del2(): Box[Unit] = req("y")(s => s.length)

@main def main(): Unit =
  println(Reqs.del().a == ())
  println(Reqs.del2().a == ())
