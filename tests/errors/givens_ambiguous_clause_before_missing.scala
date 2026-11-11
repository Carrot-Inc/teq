// expect: 14:43: error: ambiguous given instances for A: a1, a2
// expect: 1 error found
// An ambiguous argument in a candidate's first using clause is the candidate's failure before its second clause
// is searched: the lower given low is not taken.
trait A
trait Missing
trait R { def n: Int }
given a1: A with {}
given a2: A with {}
object R extends Low:
  given r(using A)(using Missing): R with { def n = 1 }
trait Low:
  given low: R with { def n = 2 }
@main def main(): Unit = println(summon[R].n)
