// lila: app/controllers/Fishnet.scala:80 (`allow: ... .rescue(identity)`, cats-mtl's `allow` and `rescue`)
// `allow`'s error type is inferred from what the body raises (`Result`), and the eta-expansion of `identity`
// against `Result => Future[Result]` is a closure typed against the expected function (`EtaExpansion.etaExpand`):
// its body `identity(x)` takes the result's conversion to `Future[Result]`.
import scala.concurrent.{ Await, Future }
import scala.concurrent.duration.*
import scala.language.implicitConversions

trait Raise[F[_], -E]
final class Wired[E]:
  def apply[F[_], A](body: Raise[F, E] ?=> F[A]): Tired[F, E, A] = Tired(body(using new Raise[F, E] {}))
final class Tired[F[_], E, A](fa: F[A]):
  def rescue(h: E => F[A]): F[A] = fa
def allow[E]: Wired[E] = Wired[E]()

case class Result(n: Int)
given Conversion[Result, Future[Result]] = Future.successful(_)

def work(using Raise[Future, Result]): Future[Result] = Future.successful(Result(7))

@main def run(): Unit =
  val f: Future[Result] =
    allow:
      work
    .rescue(identity)
  println(Await.result(f, 1.second))
