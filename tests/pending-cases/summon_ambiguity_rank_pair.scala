// A search for a value is `rank(sort(eligible))` (Implicits.scala:1663): `sort` tries `c` first (its owner `C extends Y
// extends X` has the most base classes), then `a` and `b`; `c` and `a` are ranked alike, an ambiguity `b` (below `a`)
// does not heal, so scalac 3.8.4 names c and a (Codex's q12 on push 5). master (efddb38de3), the head (61c1af7941) and
// 2f1697722f "ambiguous given instances for Ops: a, c" (a trial order by how many candidates each beats); push 5's fold
// "c, a", in teq's own message. Queued: push 6 (AmbiguousImplicits' message).
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
