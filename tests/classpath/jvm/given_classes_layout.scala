// jars: scala-library
// std: scala-library
// A given class of a package or of objects in scalac's layout:
// its class `Owner$name` and the def making one, which every summon calls, a given of type parameters alone
// too (`eitherFunctor made` twice, as under scalac).
trait Functor[F[_]]:
  def map[A, B](fa: F[A])(f: A => B): F[B]
trait Show[A]:
  def show(a: A): String
object Instances:
  given eitherFunctor[L]: Functor[[R] =>> Either[L, R]] with
    println("eitherFunctor made")
    def map[A, B](fa: Either[L, A])(f: A => B): Either[L, B] = fa.map(f)
  given listShow[A](using s: Show[A]): Show[List[A]] with
    def show(as: List[A]): String = as.map(s.show).mkString("[", ",", "]")
  given Show[Int] with
    def show(a: Int): String = s"i$a"
given optShow[A](using s: Show[A]): Show[Option[A]] with
  def show(o: Option[A]): String = o.fold("none")(s.show)
object Main:
  import Instances.given
  def main(args: Array[String]): Unit =
    val f = summon[Functor[[R] =>> Either[String, R]]]
    println(f.map(Right(1): Either[String, Int])(_ + 1))
    println(summon[Functor[[R] =>> Either[String, R]]] eq f)
    println(summon[Show[List[Int]]].show(List(1, 2)))
    println(summon[Show[Option[Int]]].show(Some(3)))
