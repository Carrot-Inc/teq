// scala-library's `NumericRange` in `scala.collection.immutable`, its factory `apply` (exclusive)
// and `inclusive`, `by`, the `Long` and `Char` ranges of `until` and `to`, and their printing.
import scala.collection.immutable.NumericRange

@main def main(): Unit =
  val r = scala.collection.immutable.NumericRange(1L, 5L, 2L)
  println(r.toList)
  val i: NumericRange.Inclusive[Long] = NumericRange.inclusive(1L, 5L, 2L)
  println(i.toList)
  println(i.exclusive.toList)
  println((1L until 4L).toList)
  println((1L to 7L by 3L).toList)
  println(('a' to 'e').mkString)
  println(('a' until 'e' by 2).toList)
  println(NumericRange(10L, 0L, -3L).toList)
  println(NumericRange(0L, 0L, 1L).isEmpty)
  println(NumericRange(1L, 10L, 3L).last)
