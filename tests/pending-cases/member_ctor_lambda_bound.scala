// A member type constructor as the argument of a parameter bounded by a lambda (`P#F` for `F[X] <: [Y] =>> Any`):
// scalac 3.8.4 compares it by eta-expansion (`TypeComparer.compareTypeLambda`), teq refuses it ("does not conform to
// upper bound [X] =>> [Y] =>> Any"), master's: with the kind helpers' other part (found by the fold of the review of
// `bb04d0aaf`).
trait P { type F[X] <: [Y] =>> Any }
type K[F[X] <: [Y] =>> Any] = Int

@main def run(): Unit =
  val k: K[P#F] = 1
  println(k)
