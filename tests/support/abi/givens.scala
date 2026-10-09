// Item 8 of the JVM ABI alignment: a given object of a package or of
// objects is scalac's module class `Owner$x$`, its `MODULE$` and public constructor, the bridges of its methods, a
// top-level one inside its file's `X$package`; its owner declares it as a public static final field and has no
// accessor; teq's reads go to the module. A given class of a package or of objects is scalac's class `Owner$name`
// with the def `name(params)` answering a new one.
// abi: classes MODULE$ given_Show_Int named given_Show_String show listShow optShow eitherFunctor map
package givens
trait Show[A] { def show(a: A): String }
object Show:
  given Show[Int] with { def show(a: Int): String = a.toString }
  given named: Show[Long] with { def show(a: Long): String = s"L$a" }
given Show[String] with { def show(a: String): String = a }
object Deep:
  object Inner:
    given Show[Boolean] with { def show(a: Boolean): String = a.toString }
object Lists:
  given listShow[A](using s: Show[A]): Show[List[A]] with { def show(as: List[A]): String = as.map(s.show).mkString(",") }
given optShow[A](using s: Show[A]): Show[Option[A]] with { def show(o: Option[A]): String = o.fold("none")(s.show) }
trait Functor[F[_]] { def map[A, B](fa: F[A])(f: A => B): F[B] }
object Functors:
  given eitherFunctor[L]: Functor[[R] =>> Either[L, R]] with { def map[A, B](fa: Either[L, A])(f: A => B): Either[L, B] = fa.map(f) }
