// The `EffDefaults` programs link, in the place of the one tests/tasty/src/effects.scala compiles its
// library against; only its own TASTy is a fixture.
package fix.shadow

trait Eff[F[_]]:
  def delay[A](a: => A): F[A]
  def run[A](fa: F[A]): A

trait EffApi:
  type F[A]
  implicit def F: Eff[F]

object EffDefaults extends EffApi:
  override type F[A] = Option[A]
  override implicit val F: Eff[F] = new Eff[F]:
    def delay[A](a: => A): Option[A] = Some(a)
    def run[A](fa: Option[A]): A = fa.get
