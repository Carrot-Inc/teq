// A stream's iterator leaves the tail after the head it returns unforced, as scala-library's
// `LinearSeqIterator` does: a throwing tail after a valid head is reached by the next `hasNext`.
@deprecated("uses Stream", "") @main def main(): Unit =
  var count = 0
  val s = Stream.cons(1, { count += 1; Stream.cons(2, Stream.empty) })
  val it = s.iterator
  println(it.next())
  println(count)
  println(it.hasNext)
  println(count)
  println(it.next())
  println(it.hasNext)
  def failing: Stream[Int] = throw new RuntimeException("tail")
  val bad = Stream.cons(1, failing)
  val it2 = bad.iterator
  println(it2.next())
  println(try it2.hasNext.toString catch case e: RuntimeException => e.getMessage)
