// scala-library's `IterableOnce` has `iterator`
// abstract and no `foreach`; the program's
// source defines its iterator alone. Queued:
// the std's `IterableOnce` as scala-library's.
import scala.collection.immutable.TreeSet
class OnlyIter(xs: List[Int]) extends IterableOnce[Int]:
  def iterator: Iterator[Int] = xs.iterator
@main def run(): Unit =
  println(TreeSet.from(new OnlyIter(List(2, 1))))
  val b = TreeSet.newBuilder[Int]
  b ++= new OnlyIter(List(3, 1))
  println(b.result())
  println(Vector.from(new OnlyIter(List(4, 5))))
