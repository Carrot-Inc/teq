// Inside a case class's companion, `apply` unqualified is the companion's synthesized
// constructor `apply`.
final case class Arg[A](render: A => String)

object Arg:
  def unit[F[_]]: Arg[F[Unit]] = apply(_ => "unit")
  def list[A](show: A => String): Arg[List[A]] = apply(_.map(show).mkString("[", ",", "]"))

case class Point(x: Int, y: Int)
object Point:
  val origin: Point = apply(0, 0)
  def diagonal(n: Int): Point = apply(n, n)

@main def run(): Unit =
  println(Arg.unit[Option].render(Some(())))
  println(Arg.list[Int](_.toString).render(List(1, 2)))
  println(s"${Point.origin} ${Point.diagonal(3)}")
