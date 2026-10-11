// A block's alias over a path (`type L = O.K`), entered where it stands, and aliases that name it
// (`type M = L`, `type N = List[M]`), declared with the block's definitions and completed at their
// statements, after it, as Namer's completers are.
object O:
  class K:
    override def toString = "K"

@main def run(): Unit =
  val k = new O.K
  type L = O.K
  type M = L
  type N = List[M]
  val m: M = k
  val n: N = List(m, m)
  println(n)
