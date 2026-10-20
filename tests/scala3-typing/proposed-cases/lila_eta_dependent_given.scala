// lila: modules/storm/src/main/StormSelector.scala:90 (`.map(ThreadLocalRandom.shuffle)`)
// The eta-expansion of a method whose given depends on its parameter (`BuildFrom[xs.type, T, C]`) takes the
// expected function's parameter type, `List[Int]`, as `EtaExpansion.etaExpand` leaves the parameters to the
// prototype where the method is the last application: the given is found for the list.
import scala.collection.BuildFrom

object R:
  def shuffle[T, C](xs: IterableOnce[T])(using bf: BuildFrom[xs.type, T, C]): C =
    bf.fromSpecific(xs)(xs.iterator.toList.reverse)

@main def run(): Unit = println(List(List(1, 2), List(3)).map(R.shuffle))
