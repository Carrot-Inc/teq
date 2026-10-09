// A range up to `Int.MaxValue` ends at it, never stepping past (scala-library's element count and
// last element, no exclusive limit); `sum` is the arithmetic series under the standard `Numeric`,
// overflowing as scala-library's formula does, and a custom `Numeric`'s `plus` per element.
object Main:
  object Counting extends Numeric[Int]:
    var plus = 0
    def plus(x: Int, y: Int): Int = { plus += 1; x + y }
    def minus(x: Int, y: Int): Int = x - y
    def times(x: Int, y: Int): Int = x * y
    override def negate(x: Int): Int = -x
    override def zero: Int = 0
    override def one: Int = 1
    def fromInt(x: Int): Int = x
    def parseString(str: String): Option[Int] = str.toIntOption
    def toInt(x: Int): Int = x
    def toLong(x: Int): Long = x
    def toFloat(x: Int): Float = x.toFloat
    def toDouble(x: Int): Double = x
    def compare(x: Int, y: Int): Int = Integer.compare(x, y)

  def main(args: Array[String]): Unit =
    println((Int.MaxValue to Int.MaxValue).sum)
    println((Int.MaxValue - 1 to Int.MaxValue).sum)
    println((Int.MaxValue - 2 to Int.MaxValue).toList)
    println((Int.MinValue to Int.MinValue + 1).toList)
    println((Int.MaxValue to Int.MaxValue - 4 by -2).toList)
    println((Int.MaxValue - 3 to Int.MaxValue by 2).length)
    var n = 0
    (Int.MaxValue - 2 to Int.MaxValue).foreach(_ => n += 1)
    println(n)
    println((Int.MaxValue - 1 to Int.MaxValue).contains(Int.MaxValue))
    println(s"${(1 to 100).sum} ${(1 until 1).sum} ${(10 to 1 by -3).sum}")
    val counted = (1 to 10).sum(using Counting)
    println(s"$counted ${Counting.plus}")
    println(s"${(1 to 10 by 3).toList} ${(1 to 10 by 3).last} ${(1 until 10 by 3).last}")
    println(s"${(1 to 10).slice(2, 5)} ${(1 to 10).takeRight(3).toList} ${(1 to 10).dropRight(8).toList}")
    println(s"${1 to 10 by 4} ${1 until 10 by 4} ${5 to 1}")
    println(s"${(0 until 10).tail.head} ${(1 to 1).tail.isEmpty} ${(3 to 9 by 3).reverse.toList}")
    val it = (Int.MaxValue - 1 to Int.MaxValue).iterator
    println(s"${it.next()} ${it.next()} ${it.hasNext}")
    try (Int.MinValue to Int.MaxValue).length catch case e: IllegalArgumentException => println(e.getMessage)
