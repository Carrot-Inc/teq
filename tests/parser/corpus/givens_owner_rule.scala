//> using scala 3.8.4

// Since Scala 3.7 the owner relation decides between two applicable givens whatever their
// types: an object's given beats one inherited from a trait even when the trait's is the
// more specific type.
trait Show[T] { def n: String }
trait LowPrio:
  given lowInt: Show[Int] with { def n = "low specific Int" }
object Show extends LowPrio:
  given highAny[T]: Show[T] with { def n = "high generic" }

trait Enc[T] { def n: String }
trait EncLow:
  given encAny[T]: Enc[T] with { def n = "low generic" }
object Enc extends EncLow:
  given encInt: Enc[Int] with { def n = "high specific Int" }

// Unrelated owners: the more general type wins, and after a draw the given without using
// parameters.
trait Ctx
given ctx: Ctx with {}
object Unrelated:
  given viaCtx[T](using Ctx): Show[T] with { def n = "via ctx" }
object Other:
  given plain: Show[String] with { def n = "plain" }

@main def main(): Unit =
  println(summon[Show[Int]].n)
  println(summon[Show[String]].n)
  println(summon[Enc[Int]].n)
  println(summon[Enc[String]].n)
  import Unrelated.given
  import Other.given
  println(summon[Show[String]].n)
