// A package context's implicits are its this type's `implicitMembers`, over
// `PackageClassDenotation#memberNames` (SymDenotations.scala:2637-2645): the package
// object `p extends p.T2, p.T1` lists T2's `b` before T1's `a`, and of one level
// and arity with equal base classes neither sorts first, so scalac 3.8.4 names
// b and a. teq is to name them so. Queued: push 6 (AmbiguousImplicits' message).
package p {
  trait Ops
  trait T1 { given a: Ops = new Ops {} }
  trait T2 { given b: Ops = new Ops {} }
  object Main {
    def main(args: Array[String]): Unit = println(summon[Ops])
  }
}
package object p extends p.T2, p.T1
