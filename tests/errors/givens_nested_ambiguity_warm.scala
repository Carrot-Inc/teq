// expect: 15:22: error: ambiguous given instances for A: a1, a2
// expect: 1 error found
// An ambiguity met while a using parameter of a candidate is resolved propagates: the search
// does not fall through to the outer given c.
trait A
trait C { def n: String }
given a1: A with {}
given a2: A with {}
given c: C with { def n = "outer c" }
object Warm:
  given a3: A with {}
  def run = summon[C].n
object O:
  given b(using A): C with { def n = "b" }
  def run = summon[C].n
@main def main(): Unit =
  println(O.run)
