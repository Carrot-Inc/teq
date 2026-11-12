// A member type constructor as the argument of a parameter bounded
// by a lambda (`P#F` for `F[X] <: [Y] =>> Any`): scalac 3.8.4
// compares it by eta-expansion (`TypeComparer.compareTypeLambda`).
// Queued with the kind helpers' other part.
trait P { type F[X] <: [Y] =>> Any }
type K[F[X] <: [Y] =>> Any] = Int

@main def run(): Unit =
  val k: K[P#F] = 1
  println(k)
