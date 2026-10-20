// Pinned for the queued part inlining-lila-folds: the commit that made this case pass (db0a8ede5, the lila folds'
// eta-expansion and context-function changes) broke tests/scala3's boehm-berarducci (scala3-run and scala3-interp)
// and pos/i12211, pos/i3945 against master (the gate of 5010f41f8), a method value eta-expanded against an expected
// function type with its variables solved to Any; reverted on 2026-10-20. scalac accepts and prints the expected
// output.
// An inline method's context function result against a body's open `F[A]`: the result is
// followed to its own result before the comparison (`ProtoTypes.necessarilyCompatible`'s
// `normalize` with `followIFT`), so `F` is the monad, not the context function's constructor,
// and the value is applied to the contextual lambda's given; the handler's colon lambda untuples
// from `E`'s lower bound (`Typer.typedFunctionValue`'s `ptIsCorrectProduct`).
trait AT[F[_]]
class Box[+A](val value: A):
  def map[B](f: A => B): Box[B] = Box(f(value))
object Box:
  def apply[A](a: A): Box[A] = new Box(a)
given AT[Box] = new AT[Box] {}
trait Raise[F[_], -E]:
  def raise[E2 <: E, A](e: E2): F[A]
trait Handle[F[_], E] extends Raise[F, E]
final class Inner[F[_], E, A](body: Handle[F, E] ?=> F[A]):
  inline def rescue(inline f: E => F[A])(using AT[F]): F[A] =
    body(using new Handle[F, E] { def raise[E2 <: E, B](e: E2): F[B] = f(e).asInstanceOf[F[B]] })
final class AdHoc[E]():
  inline def apply[F[_], A](inline body: Handle[F, E] ?=> F[A]): Inner[F, E, A] = new Inner[F, E, A](body)
inline def allow[E]: AdHoc[E] = new AdHoc[E]()
def prepare(x: Int): Raise[Box, (Int, String)] ?=> Box[Int] =
  if x > 0 then Box(x) else summon[Raise[Box, (Int, String)]].raise((429, "no"))
def run[A](key: String, default: => Box[A])(f: => Box[A]): Box[A] = f
def test(x: Int) =
  allow:
    run[Int]("k", Box(0)):
      prepare(x)
  .rescue: (code, msg) =>
    Box(code + msg.length)
object Main:
  def main(args: Array[String]): Unit =
    println(test(5).value)
    println(test(-1).value)
