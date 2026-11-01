// scala-library's `IterableOnce` has `iterator` abstract and no `foreach`; the std's declares `foreach` abstract and
// derives `iterator` from it, so a source defining its iterator alone is refused, and the std's own walks of a source
// (`iterableToArray`, the sorted collections' `from` and `concat`) take its `foreach`. Queued: the std's
// `IterableOnce` as scala-library's.
import scala.collection.immutable.TreeSet
class OnlyIter(xs: List[Int]) extends IterableOnce[Int]:
  def iterator: Iterator[Int] = xs.iterator
@main def run(): Unit =
  println(TreeSet.from(new OnlyIter(List(2, 1))))
  val b = TreeSet.newBuilder[Int]
  b ++= new OnlyIter(List(3, 1))
  println(b.result())
  println(Vector.from(new OnlyIter(List(4, 5))))
