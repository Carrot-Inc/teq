//> using platform jvm
// A case class's own `copy` with a default: the user's method and its default getter are the class's members,
// the synthetic ones are never made (dotty's `Namer.invalidateIfClashingSynthetic`, Namer.scala 927-965).
case class Bump(x: Int) {
  def copy(x: Int = 7): Bump = new Bump(x + 1)
}
@main def run(): Unit =
  println(Bump(2).copy().x)
  println(Bump(2).copy(4).x)
