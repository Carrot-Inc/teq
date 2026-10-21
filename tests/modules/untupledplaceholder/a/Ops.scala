// An untupled placeholder lambda over a list of pairs (`xs.map(_ + _)`): the Scala.js bundle of the split build reads
// each element as `(t$0._1 ?? 0)`, the whole build's as `t$0._1` (master's; a part of its own, `known.txt`).
package pa

object Ops:
  def sum(xs: List[(Int, Int)]): List[Int] = xs.map(_ + _)
