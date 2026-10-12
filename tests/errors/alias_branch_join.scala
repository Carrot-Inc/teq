// The branches of an `if` whose applications leave a variable open under a plain alias
// (`IO[?A]` of `type IO[A] = Fr[Op, A]`, nonvariant in `Fr`): the alias stands for its expansion,
// so the variables stay open for the join, which unifies them as dotty's `widenUnion` does
// (`lub(.., canConstrain = true)`), and the value is an `IO[Either[String, Int]]` as under scalac.
class Fr[S[_], A](val a: A)
trait Op[A]
type IO[A] = Fr[Op, A]
def pure[A](a: A): IO[A] = new Fr(a)
def run(c: Boolean): Unit =
  val joined = if c then pure(Left("no")) else pure(Right(1))
  val t: Int = joined
// expect: 11:16: error: type mismatch: found IO[Either[String, Int]], required Int
