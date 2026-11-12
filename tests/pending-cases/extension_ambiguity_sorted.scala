// The givens in scope are tried in `sort`'s order (`compareEligibles`,
// Implicits.scala:1584-1595): of one level and arity, the one whose
// owner has more base classes first, so the ambiguity, the qualifier's
// mismatch (Typer.scala:4346-4353), names `b` (of `T2`, which extends
// `Extra`) before `a` (scalac 3.8.4). Queued: push 6 (typeMismatch whole).
class R
trait Ops:
  extension (r: R) def pick: String
trait Extra
trait T1:
  given a: Ops with
    extension (r: R) def pick: String = "a"
trait T2 extends Extra:
  given b: Ops with
    extension (r: R) def pick: String = "b"
object Main extends T1, T2:
  def main(args: Array[String]): Unit = println((new R).pick)
