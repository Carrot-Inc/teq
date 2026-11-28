//> using platform jvm
// A value case class's own `copy` without a default: the companion's `copy$extension` is the user's.
case class Wrapped(x: Int) extends AnyVal {
  def copy(x: Int): Wrapped = new Wrapped(x + 1)
}
@main def run(): Unit = println(Wrapped(2).copy(8).x)
