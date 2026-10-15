// A stream is an `AbstractSeq` and a `LinearSeq`, as scala-library's `Stream` extends them, and its
// operations stay streams: the base type of `SeqOps` and `IterableOps` is the meet of the parents'
// (`Stream`'s own beside `Seq`'s).
@deprecated("uses Stream", "")
@main def main(): Unit =
  val s: scala.collection.AbstractSeq[Int] = Stream(1, 2)
  println(s.head)
  val l: scala.collection.immutable.LinearSeq[Int] = Stream(3, 4)
  println(l.length)
  val t: Stream[Int] = Stream(2, 1) ++ Stream(3)
  val u: Stream[Int] = t.sorted
  val v: Stream[Int] = u.updated(0, 9)
  val w: Stream[Int] = v.map(_ + 1).filter(_ > 2)
  println((t.toList, u.toList, v.toList, w.toList))
