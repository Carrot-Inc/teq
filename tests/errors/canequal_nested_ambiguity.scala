// expect: 14:40: error: ambiguous given instances for M: m1, m2
// expect: 1 error found
// A CanEqual search takes the first candidate that applies, in order (dotc's coherent search),
// but a candidate before it that failed with a nested ambiguity stands unless the later one is
// strictly better: `a` and `b` are alike, so the ambiguity of `a`'s `M` is reported.
object Test:
  trait M
  trait N
  given m1: M = new M {}
  given m2: M = new M {}
  given n: N = new N {}
  given a(using M): CanEqual[String, Int] = CanEqual.derived
  given b(using N): CanEqual[String, Int] = CanEqual.derived
  val x = summon[CanEqual[String, Int]]
