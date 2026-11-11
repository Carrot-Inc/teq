import scala.language.implicitConversions

class Wrap[F[_]]
class Box[A]
class Ops[F[_], A](value: F[A]):
  def nested: Box[Wrap[[X] =>> F[X]]] = new Box
object Ops:
  implicit def wrap[F[_], A](value: F[A]): Ops[F, A] = new Ops(value)
object Probe:
  import Ops.*
  def choose[F[_]](f: F[Int] => F[Int]): Int = 0
  def choose(f: Box[Int] => Box[Wrap[[X] =>> Box[X]]])(using DummyImplicit): String = "nested"
  // The generic candidate would require F[Int] to contain Box[Wrap[F]].
  // Its speculative constraints must not survive choosing the concrete overload.
  def result: String = choose(x => x.nested)
@main def run(): Unit = println(Probe.result)
