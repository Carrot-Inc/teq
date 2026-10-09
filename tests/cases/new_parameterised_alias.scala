// `new A[T](args)` for an alias that fixes some of its class's type arguments constructs the
// class at the alias's expansion.
class Scoped[F[_], P](val p: P):
  def show: String = s"scoped($p)"
  def wrap(f: P => F[P]): F[P] = f(p)

type OptScope[P] = Scoped[Option, P]
type Swapped[P, F[_]] = Scoped[F, P]
type Plain = Scoped[List, Int]

@main def run(): Unit =
  val a = new OptScope[Int](3)
  println(a.show)
  println(a.wrap(Some(_)))
  val b: Scoped[List, String] = new Swapped[String, List]("x")
  println(b.wrap(List(_, "y")))
  println(new Plain(4).show)
