// An extension method of a type class can be called through the instance, `F.map(fa)(f)`, as
// well as on the value, `fa.map(f)`, when the instance is a given in scope.
trait Functor[F[_]]:
  extension [A](fa: F[A]) def map[B](f: A => B): F[B]
  extension [A](fa: F[A]) def as[B](b: B): F[B] = fa.map(_ => b)

trait Show[A]:
  extension (a: A) def show: String
  extension (a: A) def shown(prefix: String): String = prefix + a.show

final case class Slug[T](value: T)
given Functor[Slug] with
  extension [A](fa: Slug[A]) def map[B](f: A => B): Slug[B] = Slug(f(fa.value))

given Show[Int] with
  extension (a: Int) def show: String = s"#$a"

def lift[F[_], A, B](fa: F[A])(f: A => B)(using F: Functor[F]): F[B] = F.map(fa)(f)
def lift2[F[_]: Functor, A, B](fa: F[A])(f: A => B): F[B] = fa.map(f)
def replace[F[_], A](fa: F[A])(using F: Functor[F]): F[String] = F.as(fa)("done")

@main def main(): Unit =
  println(lift(Slug(1))(_ + 1))
  println(lift2(Slug(1))(_ + 2))
  println(Slug(2).map(_ * 2))
  println(summon[Functor[Slug]].map(Slug(5))(_ + 1))
  println(replace(Slug(3)))
  val functor = summon[Functor[Slug]]
  println(functor.as(Slug(4))(true))
  val show = summon[Show[Int]]
  println(show.show(7))
  println(show.shown(8)("n="))
  println(9.shown("m="))
