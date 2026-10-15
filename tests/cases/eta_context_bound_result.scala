// A method with a context bound eta-expanded where a function is expected: its result is the expected
// function's before its using clause is searched (dotty constrains a result by its prototype ahead of the
// implicit arguments), as tyrian's laws test expands `Sub.emit[F[_]: Sync, Msg]` into an `A => Sub[IO, A]`.
trait Sync[F[_]]
object Sync:
  given Sync[List] with {}
  given Sync[Option] with {}
final case class Sub[F[_], A](a: A)
object Sub:
  def emit[F[_]: Sync, Msg](msg: Msg): Sub[F, Msg] = Sub(msg)
final case class Gen[A](a: A):
  def map[B](f: A => B): Gen[B] = Gen(f(a))
@main def run(): Unit =
  more()
  val x: Gen[Sub[List, Int]] = Gen(3).map(Sub.emit)
  val f: Int => Sub[Option, Int] = Sub.emit
  println(x)
  println(f(4))
final case class Arbitrary[A](g: Gen[A])
def arbSub[A](g: Gen[A]): Arbitrary[Sub[List, A]] = Arbitrary(g.map(Sub.emit))
def more(): Unit = println(arbSub(Gen("s")))
