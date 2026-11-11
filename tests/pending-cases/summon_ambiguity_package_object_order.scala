// A package context's implicits are its this type's `implicitMembers`, over `PackageClassDenotation#memberNames`
// (SymDenotations.scala:2637-2645): the package object `p extends p.T2, p.T1` lists T2's `b` before T1's `a`, and of
// one level and arity with equal base classes neither sorts first, so scalac 3.8.4 names b and a (Codex's q13 on push
// 5). master (efddb38de3), the head (61c1af7941) and 2f1697722f "ambiguous given instances for Ops: a, b" (the package
// index sorted by symbol); push 5's fold "b, a", in teq's own message. Queued: push 6 (AmbiguousImplicits' message).
package p {
  trait Ops
  trait T1 { given a: Ops = new Ops {} }
  trait T2 { given b: Ops = new Ops {} }
  object Main {
    def main(args: Array[String]): Unit = println(summon[Ops])
  }
}
package object p extends p.T2, p.T1
