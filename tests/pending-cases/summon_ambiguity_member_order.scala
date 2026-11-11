// A class context's search for a value tries its givens in `implicitMembers`' order, which `sort` keeps for ties
// (Implicits.scala:1648-1660; `computeMemberNames`, SymDenotations.scala:2409-2425): scalac 3.8.4 names a before b in
// `M1 extends T1, T2` and b before a in `M2 extends T2, T1` (AmbiguousImplicits' message). master (efddb38de3) and the
// head (61c1af7941) "ambiguous given instances for Ops: a, b" for both; push 5 "a, b" and "b, a". The head and push 5
// add "Note: method a in trait T1 was not considered because it was not imported with `import given`" for a member the
// class context holds, which neither scalac nor master prints: a TR regression. Queued: push 6 (AmbiguousImplicits'
// message), the note with it.
trait Ops
trait T1:
  given a: Ops = new Ops {}
trait T2:
  given b: Ops = new Ops {}
object M1 extends T1, T2:
  def run(): Ops = summon[Ops]
object M2 extends T2, T1:
  def run(): Ops = summon[Ops]
object Main:
  def main(args: Array[String]): Unit = println(M2.run())
