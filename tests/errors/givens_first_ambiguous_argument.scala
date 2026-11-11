// expect: 13:23: error: ambiguous given instances for A: a1, a2
// expect: 1 error found
// Of two ambiguous arguments of one clause the first is the candidate's failure (propagatedFailure, Typer.scala:
// 4603-4610): scalac reports a1 and a2.
trait A
trait B
trait R
given a1: A with {}
given a2: A with {}
given b1: B with {}
given b2: B with {}
given r(using A, B): R with {}
val result = summon[R]
