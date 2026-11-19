// A view keeps scala-library's type: a `View` is no `SeqView`, a `SeqView` no `IndexedSeqView`,
// and `filter` gives a `View` where `map` keeps the kind.
// expect: 11:25: error: Found:    View[Int]⏎Required: SeqView[Int]
// expect: 12:32: error: Found:    SeqView[Int]⏎Required: IndexedSeqView[Int]
// expect: 13:25: error: Found:    View[Int]⏎Required: SeqView[Int]
// expect: 14:32: error: Found:    View[Int]⏎Required: IndexedSeqView[Int]
// expect: 4 errors found
import scala.collection.{View, SeqView, IndexedSeqView}
object Main:
  val v: View[Int] = List(1).view
  val s: SeqView[Int] = v
  val i: IndexedSeqView[Int] = List(1).view
  val f: SeqView[Int] = List(1).view.filter(_ > 0)
  val g: IndexedSeqView[Int] = Vector(1).view.flatMap(x => List(x))
  val ok: SeqView[Int] = List(1).view.map(_ + 1)
