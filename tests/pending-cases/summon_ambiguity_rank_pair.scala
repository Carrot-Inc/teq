// A search for a value is `rank(sort(eligible))` (Implicits.scala:1663):
// `sort` tries `c` first (its owner `C extends Y extends X` has the most
// base classes), then `a` and `b`; `c` and `a` are ranked alike, an
// ambiguity `b` (below `a`) does not heal, so scalac 3.8.4 names c and a.
// teq is to name them so. Queued: push 6 (AmbiguousImplicits' message).
trait Ops
trait B:
  given b: Ops = new Ops {}
trait A extends B:
  given a: Ops = new Ops {}
trait X
trait Y extends X
trait C extends Y:
  given c: Ops = new Ops {}
object Main extends A, C:
  def main(args: Array[String]): Unit = println(summon[Ops])
