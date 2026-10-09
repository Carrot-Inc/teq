// The deprecated `toIterator`, which a library compiled against it still calls: the elements
// once, in order.
@main def run =
  val xs: Seq[Int] = Seq(1, 2, 3)
  val it = xs.toIterator
  println(it.next())
  println(it.toList)
  println(Set("a").toIterator.hasNext)
