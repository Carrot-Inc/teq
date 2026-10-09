// `compare` of scala-library's rich wrappers on Int, Long and Double (zio's `DurationOps.compare`
// calls `longWrapper(x).compare(y)`), and `Iterator.distinct`, which keeps the first of each.
object Main:
  def main(args: Array[String]): Unit =
    println(s"${3.compare(4)} ${4.compare(4)} ${Int.MaxValue.compare(Int.MinValue)}")
    println(s"${5L.compare(3L)} ${Long.MinValue.compare(0L)}")
    println(s"${2.5.compare(2.5)} ${(-0.0).compare(0.0)} ${Double.NaN.compare(1.0)} ${1.0.compare(Double.NaN)}")
    println(Iterator(1, 2, 1, 3, 2).distinct.toList)
    println(Iterator("aa", "b", "cc", "d").distinctBy(_.length).toList)
