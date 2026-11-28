//> using platform jvm
// A value case class's own `copy` with a default: its extension method and the default getter's are the user's.
case class Boxed(x: Int) extends AnyVal {
  def copy(x: Int = 7): Boxed = new Boxed(x + 1)
}
@main def run(): Unit =
  println(Boxed(2).copy().x)
  println(Boxed(2).copy(8).x)
