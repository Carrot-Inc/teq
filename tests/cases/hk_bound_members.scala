// A member selected on an applied higher-kinded type parameter goes through the parameter's
// bound instantiated at the arguments: `ev: S[A]` under `S[T] <: Semi[T]` has `Semi[A]`'s
// members, conforms to `Semi[A]` and answers a search for `Semi[A]`; under `F2[X] >: F[X]` an
// `F[A]` conforms to `F2[A]`.
trait Semi[A]:
  def combine(x: A, y: A): A
trait Comm[A] extends Semi[A]:
  def name: String = "comm"

trait Funcs[S[T] <: Semi[T]]:
  def maybeCombine[A](ox: Option[A], y: A)(implicit ev: S[A]): A = ox match
    case Some(x) => ev.combine(x, y)
    case None => y
  def widen[A](implicit ev: S[A]): Semi[A] = ev
  def summoned[A](implicit ev: S[A]): Semi[A] = implicitly[Semi[A]]
  def twice[A](x: A)(implicit ev: S[A]): A = widen(using ev).combine(x, x)

trait CommFuncs[S[T] <: Comm[T]] extends Funcs[S]:
  def nameOf[A](implicit ev: S[A]): String = ev.name

given Semi[Int] with
  def combine(x: Int, y: Int): Int = x + y
given Comm[String] with
  def combine(x: String, y: String): String = x + y

object F extends Funcs[Semi]
object C extends CommFuncs[Comm]

class Lift[F[_], F2[X] >: F[X]]:
  def up[A](fa: F[A]): F2[A] = fa
  def ev[A](using F[A] <:< F2[A]): String = "evidence"

class Holder[F[X] <: Iterable[X]](val items: F[Int]):
  def total: Int = items.foldLeft(0)(_ + _)
  def first: Option[Int] = items.headOption

@main def run(): Unit =
  println(F.maybeCombine(Some(1), 2))
  println(F.maybeCombine(None, 2))
  println(F.twice(5))
  println(F.summoned[Int].combine(3, 4))
  println(C.nameOf[String])
  println(C.maybeCombine(Some("a"), "b"))
  println(Holder[List](List(1, 2, 3)).total)
  println(Holder[Vector](Vector(4, 5)).first)
  println(Lift[Some, Option]().up(Some(1)))
  println(Lift[Some, Option]().ev[Int])
