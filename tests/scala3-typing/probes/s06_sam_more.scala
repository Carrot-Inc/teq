trait FromString[A]:
  def parse(s: String): A
given FromString[Int] = _.toInt
def singleton[A](a: A): Set[A] = _ == a
object F:
  def pure[B](b: B): Option[B] = Some(b)
case class Kleisli[F[_], A, B](run: A => F[B])
def k[B](b: B) = Kleisli(_ => F.pure(b))
@main def run(): Unit =
  println(summon[FromString[Int]].parse("12"))
  println(singleton(1)(1))
  Nil.foreach(x => println(x))
