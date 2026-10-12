// LazyList is a Seq and its companion a SeqFactory, as scala-library's are: `to(LazyList)`, an
// element by index and the length force the cells up to them.
def total(xs: Seq[Int]): Int = xs.sum

@main def main(): Unit =
  var forced = 0
  val ll = LazyList.from(1).map { n => forced += 1; n * n }
  println(ll(3))
  println(forced)
  val five = ll.take(5)
  println(five.length)
  println(total(five))
  val converted = List(3, 1, 2).to(LazyList)
  println(converted.toList)
  println(converted.sorted.toList)
