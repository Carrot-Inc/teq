//> using scala 3.8.4

// A candidate whose using parameter is ambiguous loses to a candidate of the same scope that
// beats it outright: c takes no using parameters, so the ambiguity of A is never reported.
trait A
trait C { def n: String }
trait B extends C
given a1: A with {}
given a2: A with {}
given b(using A): B with { def n = "b" }
given c: C with { def n = "c" }

// The same with the better candidate defined in an object that the other's owner extends.
trait Show[T] { def n: String }
trait Ctx
given ctx1: Ctx with {}
given ctx2: Ctx with {}
trait ShowLow:
  given viaCtx[T](using Ctx): Show[T] with { def n = "via ctx" }
object Show extends ShowLow:
  given plain[T]: Show[T] with { def n = "plain" }

@main def main(): Unit =
  println(summon[C].n)
  println(summon[Show[Int]].n)
