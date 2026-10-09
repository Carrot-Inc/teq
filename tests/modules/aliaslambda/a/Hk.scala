package ala

// A type alias of parameters passed as a type constructor, which teq's typer holds as its
// lambda `[A] =>> Option[A]`: read back from the products as that lambda, not as `Option`.
object Eff:
  type Async[A] = Option[A]

class Root[F[_], A[_]](val name: String)

object Defs:
  type Memo[P] = Root[List, Eff.Async]
  def memo[P](name: String): Memo[P] = new Root(name)
