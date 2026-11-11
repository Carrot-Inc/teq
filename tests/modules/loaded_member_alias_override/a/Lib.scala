package lmao.lib

trait Box[F[_], T]:
  def name: String

trait Aliased[TC[_]]:
  type Member[T] = TC[T]

trait Abstract:
  type Member[T]
  def make[A](m: String): Box[Member, A] = new BoxOf[Member, A](m)
  protected inline def inlined[A](m: String): Box[Member, A] = make[A](m)

class BoxOf[F[_], T](val name: String) extends Box[F, T]

trait Both[TC[_]] extends Aliased[TC] with Abstract:
  def split[T](box: Box[Member, T]): TC[T]
  def derived[A](m: String): Member[A] = split(make[A](m))
