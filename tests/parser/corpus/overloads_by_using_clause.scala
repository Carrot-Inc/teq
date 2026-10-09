// An overloaded name inherited from two unrelated traits, one alternative with a using clause
// and one without, each overridden by the class: scalac tells them apart by their parameter
// lists, and the output names the alternatives apart the same way.
trait Invariant[F[_]]:
  def compose[G[_]: Invariant]: Invariant[[X] =>> F[G[X]]] = new Invariant[[X] =>> F[G[X]]] {}
  def name: String = "Invariant"
trait Functor[F[_]] extends Invariant[F]:
  def map[A, B](fa: F[A])(f: A => B): F[B]
  def compose[G[_]: Functor]: Functor[[X] =>> F[G[X]]] =
    val outer = this
    new Functor[[X] =>> F[G[X]]]:
      def map[A, B](fa: F[G[A]])(f: A => B): F[G[B]] = outer.map(fa)(ga => summon[Functor[G]].map(ga)(f))
      override def name = "Functor"
trait Applicative[F[_]] extends Functor[F]:
  def pure[A](a: A): F[A]
  def compose[G[_]: Applicative]: Applicative[[X] =>> F[G[X]]] =
    val outer = this
    new Applicative[[X] =>> F[G[X]]]:
      def pure[A](a: A): F[G[A]] = outer.pure(summon[Applicative[G]].pure(a))
      def map[A, B](fa: F[G[A]])(f: A => B): F[G[B]] = outer.map(fa)(ga => summon[Applicative[G]].map(ga)(f))
      override def name = "Applicative"
trait SemigroupK[F[_]]:
  def combineK[A](x: F[A], y: F[A]): F[A]
  def compose[G[_]]: SemigroupK[[X] =>> F[G[X]]] =
    val outer = this
    new SemigroupK[[X] =>> F[G[X]]]:
      def combineK[A](x: F[G[A]], y: F[G[A]]): F[G[A]] = outer.combineK(x, y)
  def kind: String = "SemigroupK"
given Applicative[Option] with
  def pure[A](a: A): Option[A] = Some(a)
  def map[A, B](fa: Option[A])(f: A => B): Option[B] = fa.map(f)
given Applicative[List] with
  def pure[A](a: A): List[A] = List(a)
  def map[A, B](fa: List[A])(f: A => B): List[B] = fa.map(f)
final class Both extends Applicative[Option], SemigroupK[Option]:
  def pure[A](a: A): Option[A] = Some(a)
  def map[A, B](fa: Option[A])(f: A => B): Option[B] = fa.map(f)
  def combineK[A](x: Option[A], y: Option[A]): Option[A] = x.orElse(y)
  override def compose[G[_]: Applicative]: Applicative[[X] =>> Option[G[X]]] = super[Applicative].compose[G]
  override def compose[G[_]]: SemigroupK[[X] =>> Option[G[X]]] = super[SemigroupK].compose[G]
final class Plain extends Applicative[Option], SemigroupK[Option]:
  def pure[A](a: A): Option[A] = Some(a)
  def map[A, B](fa: Option[A])(f: A => B): Option[B] = fa.map(f)
  def combineK[A](x: Option[A], y: Option[A]): Option[A] = y.orElse(x)
object Main:
  def main(args: Array[String]): Unit =
    val b = Both()
    val ap: Applicative[[X] =>> Option[List[X]]] = b.compose[List]
    val sk: SemigroupK[[X] =>> Option[List[X]]] = b.compose[List]
    println(ap.pure(1).toString + " " + ap.map(Some(List(1, 2)))(_ + 1) + " " + ap.name)
    println(sk.combineK(None, Some(List(3))).toString + " " + sk.kind)
    val p = Plain()
    val pap: Applicative[[X] =>> Option[List[X]]] = p.compose[List]
    val psk: SemigroupK[[X] =>> Option[List[X]]] = p.compose[List]
    println(pap.pure(2).toString + " " + psk.combineK(Some(List(1)), Some(List(2))) + " " + pap.name)
    val f: Functor[Option] = b
    println(f.compose[List].map(Some(List(1)))(_ * 10).toString + " " + f.compose[List].name)
    val i: Invariant[Option] = b
    println(i.compose[List].name)
