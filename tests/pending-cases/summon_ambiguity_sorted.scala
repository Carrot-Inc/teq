// A search for a value tries the eligible candidates in `sort`'s order (`compareEligibles`,
// Implicits.scala:1584-1595): of one level and arity, the one whose owner has
// more base classes first, so scalac 3.8.4 names `b` (of `T2`, which extends
// `Extra`) before `a`. teq is to name them so, and to add no "was not imported with
// `import given`" note for a member the class context holds, which scalac does not
// print (summon_ambiguity_member_order). Queued: push 6 (AmbiguousImplicits' message).
trait Ops
trait Extra
trait T1:
  given a: Ops = new Ops {}
trait T2 extends Extra:
  given b: Ops = new Ops {}
object M extends T1, T2:
  def run(): Ops = summon[Ops]
object Main:
  def main(args: Array[String]): Unit = println(M.run())
