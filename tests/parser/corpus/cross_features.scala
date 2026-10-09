//> using platform js
trait Show[A]:
  def show(a: A): String
object Show:
  def apply[A](using s: Show[A]): Show[A] = s

trait Monad[F[_]]:
  def pure[A](a: A): F[A]
  def flatMap[A, B](fa: F[A])(f: A => F[B]): F[B]

case class Zone(id: Int, name: String)

trait Router:
  def route: PartialFunction[Int, String]

trait Greeter:
  def greet(s: String): String

trait Shape:
  def area: Double

def eitherMonad[E]: Monad[[A] =>> Either[E, A]] = new Monad[[A] =>> Either[E, A]]:
  def pure[A](a: A): Either[E, A] = Right(a)
  def flatMap[A, B](fa: Either[E, A])(f: A => Either[E, B]): Either[E, B] = fa.flatMap(f)

def describe(pf: PartialFunction[Int, String], xs: List[Int]): String = xs.collect(pf).mkString(",")

@main def main(): Unit =
  // partial-function literal as a vararg element typed from the expected type
  val pfs: List[PartialFunction[Int, Int]] = List({ case 1 => 10 }, { case 2 => 20 })
  println(pfs.map(_.isDefinedAt(1)))
  println(List(1, 2, 3).collect(pfs.head))

  // if branches under an open result of a type-lambda application
  val m = eitherMonad[String]
  println(m.flatMap(m.pure(3))(x => if x > 2 then Left("big") else Right(x)))
  println(m.flatMap(m.pure(1))(x => if x > 2 then Left("big") else Right(x)))

  // anonymous class members returning partial functions
  val g = new Greeter with Router:
    val prefix = "hi "
    def greet(s: String): String = prefix + s
    def route: PartialFunction[Int, String] = { case 1 => greet("one") }
  println(g.greet("x"))
  println(g.route.lift(1))
  println(g.route.lift(2))

  // anonymous class with an 8-tuple and eta-expanded companion apply
  val zones = List(1, 2).map(Zone(_, "z"))
  val mk: (Int, String) => Zone = Zone.apply
  println(zones ++ List(mk(3, "w")))
  val t = (1, 2, 3, 4, 5, 6, 7, 8)
  val s = new Shape:
    def area: Double = t._8 * 1.5
  println(s.area)

  // union of anonymous class types across branches
  val u = if List(1).size > 5 then new Greeter { def greet(s: String) = s } else new Shape { def area = 2.0 }
  println(u match
    case sh: Shape => s"shape ${sh.area}"
    case gr: Greeter => gr.greet("g"))

  // collect with an inferred union element type and a placeholder lambda through curried
  val mixed = List(Some(1), None, Some(3))
  println(mixed.collect { case Some(x) => x })
  val add3 = (a: Int, b: Int, c: Int) => a + b + c
  println(add3.curried(1)(2)(3))
  println(add3.tupled((1, 2, 3)))

  // summoner sugar with a given from an anonymous class
  given Show[Int] = new Show[Int]:
    def show(a: Int) = s"<$a>"
  println(Show[Int].show(5))

  // getOrElse joins with partial functions
  val opt: Option[PartialFunction[Int, Int]] = None
  println(opt.getOrElse(PartialFunction.empty).isDefinedAt(1))
  println(describe({ case x if x % 2 == 0 => s"e$x" }, List(1, 2, 3, 4)))
