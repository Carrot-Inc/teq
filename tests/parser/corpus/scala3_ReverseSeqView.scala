// Adapted from scala3 tests/run/ReverseSeqView.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  val lstv = List(1, 2, 3).view // SeqView
  val lstvr = lstv.reverse      // Can reverse a SeqView, but get a plain View which can no longer be reversed
  assert(lstvr.iterator.toList == List(3, 2, 1))
}
