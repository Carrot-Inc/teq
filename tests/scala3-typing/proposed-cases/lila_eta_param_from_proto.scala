// lila: storm/StormSelector.scala:90 (`puzzles.grouped(n).toList.take(k).map(ThreadLocalRandom.shuffle)`)
// Eta-expanding `shuffle` against the expected `List[Int] => ?` types its parameter as List[Int],
// so the dependent `BuildFrom[xs.type, T, C]` resolves (to List[Int]).
import scala.collection.BuildFrom

object ThreadLocalRandom:
  def shuffle[T, C](xs: IterableOnce[T])(using bf: BuildFrom[xs.type, T, C]): C =
    bf.fromSpecific(xs)(xs.iterator.toList.reverse)

@main def run() =
  val groups = List(1, 2, 3, 4).grouped(2).toList.map(ThreadLocalRandom.shuffle)
  println(groups)
