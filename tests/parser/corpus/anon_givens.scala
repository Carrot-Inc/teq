//> using scala 3.8.4

trait Show[T]:
  def show(t: T): String

trait Functor[F[_]]:
  def map[A, B](fa: F[A])(f: A => B): F[B]

trait Monad[F[_]] extends Functor[F]:
  def pure[A](a: A): F[A]
  def flatMap[A, B](fa: F[A])(f: A => F[B]): F[B]
  def map[A, B](fa: F[A])(f: A => B): F[B] = flatMap(fa)(a => pure(f(a)))

trait FieldCodec[T]:
  def encode(t: T): String
  def decode(s: String): Option[T]

case class Wrapper[A](items: List[A])
case class Pair[A, B](a: A, b: B)

object Show:
  given Show[Int] = new Show[Int]:
    def show(t: Int): String = s"i$t"
  given Show[String] with
    def show(t: String): String = s"'$t'"
  given [T: Show] => Show[List[T]] = new Show[List[T]]:
    def show(t: List[T]): String = t.map(summon[Show[T]].show).mkString("[", ",", "]")
  given [T](using s: Show[T]): Show[Option[T]] = new Show[Option[T]]:
    def show(t: Option[T]): String = t.fold("-")(v => s"?${s.show(v)}")
  given [A: Show, B: Show] => Show[Pair[A, B]] = new Show:
    def show(t: Pair[A, B]): String = s"(${summon[Show[A]].show(t.a)}, ${summon[Show[B]].show(t.b)})"

object Wrapper:
  given Functor[Wrapper] = new Functor[Wrapper]:
    def map[A, B](fa: Wrapper[A])(f: A => B): Wrapper[B] = Wrapper(fa.items.map(f))

object Instances:
  given eitherMonad[E]: Monad[[A] =>> Either[E, A]] = new Monad[[A] =>> Either[E, A]]:
    def pure[A](a: A): Either[E, A] = Right(a)
    def flatMap[A, B](fa: Either[E, A])(f: A => Either[E, B]): Either[E, B] = fa.flatMap(f)
  given optionMonad: Monad[Option] = new Monad[Option]:
    def pure[A](a: A): Option[A] = Some(a)
    def flatMap[A, B](fa: Option[A])(f: A => Option[B]): Option[B] = fa.flatMap(f)

object FieldCodec:
  given FieldCodec[Int] = new FieldCodec:
    def encode(t: Int): String = t.toString
    def decode(s: String): Option[Int] = s.toIntOption
  given [T: FieldCodec] => FieldCodec[Wrapper[T]] = new FieldCodec:
    private val inner = summon[FieldCodec[T]]
    def encode(t: Wrapper[T]): String = t.items.map(inner.encode).mkString(";")
    def decode(s: String): Option[Wrapper[T]] =
      val parts = if s.isEmpty then List() else s.split(";").toList
      val decoded = parts.map(inner.decode)
      if decoded.forall(_.isDefined) then Some(Wrapper(decoded.map(_.get))) else None

def show[T](t: T)(using s: Show[T]): String = s.show(t)

def lift[F[_]](x: Int)(using m: Monad[F]): F[Int] = m.map(m.pure(x))(_ + 1)

def roundTrip[T: FieldCodec](t: T): String =
  val c = summon[FieldCodec[T]]
  val encoded = c.encode(t)
  s"$encoded -> ${c.decode(encoded)}"

@main def run(): Unit =
  println(show(1))
  println(show("a"))
  println(show(List(1, 2)))
  println(show(Option(List("x"))))
  println(show(Pair(1, List("p"))))
  println(summon[Functor[Wrapper]].map(Wrapper(List(1, 2)))(_ * 2))
  import Instances.given
  println(lift[Option](1))
  println(lift[[A] =>> Either[String, A]](2))
  val m = summon[Monad[[A] =>> Either[String, A]]]
  println(m.flatMap(m.pure(3))(x => if x > 2 then Left[String, Int]("big") else Right[String, Int](x)))
  println(roundTrip(7))
  println(roundTrip(Wrapper(List(1, 2, 3))))
  println(roundTrip(Wrapper(List[Int]())))
  println(summon[FieldCodec[Wrapper[Int]]].decode("1;x"))
