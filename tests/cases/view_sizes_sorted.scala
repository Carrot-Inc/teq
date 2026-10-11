// A view's size as scala-library knows it: an indexed view's is its length (`IndexedSeqOps`), so
// comparing it or an adapter's reads no element, and the adapters keep their sizes; a sorted view
// sorts nothing until it is traversed, sorted again by the reverse of its order it shares the
// sorted elements (`ReverseSorted`), by another order the unsorted source.
import scala.collection.IndexedSeqView

class V extends IndexedSeqView[Int]:
  def length: Int = 3
  def apply(i: Int): Int = { print("read" + i + " "); i }

@main def run(): Unit =
  val v = new V
  println(v.lengthCompare(2))
  println(v.take(2).lengthCompare(1))
  println(List(v.knownSize, v.map(_ + 1).knownSize, v.take(1).knownSize, v.appended(5).knownSize))
  val w = Vector(1, 2, 3).view
  println(List(w.knownSize, w.map(_ + 1).knownSize, w.take(2).knownSize, w.drop(1).knownSize, w.takeRight(1).knownSize,
    w.dropRight(1).knownSize, w.appended(4).knownSize, w.prepended(0).knownSize))
  var hits = 0
  val source = List(3, 1, 2).view.map(x => { hits += 1; x })
  val first = source.sorted
  val second = first.sorted(using Ordering.Int.reverse)
  val third = first.sorted(using Ordering.by[Int, Int](x => -x))
  println("constructed " + hits)
  println(second.toList)
  println("traversed " + hits)
  println(first.toList)
  println("again " + hits)
  println(third.toList)
  println("third " + hits)
  println(second.reverse eq first)
  println(Ordering.Int.reverse.isReverseOf(Ordering.Int))
  println(Ordering.Int.reverse.reverse == Ordering.Int)
