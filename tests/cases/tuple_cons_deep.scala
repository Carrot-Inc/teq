object Test:
  def main(args: Array[String]): Unit =
    val tuple = 1 *: 2 *: 3 *: 4 *: 5 *: 6 *: 7 *: 8 *: 9 *: 10 *: 11 *: 12 *: 13 *: 14 *: 15 *: 16 *: 17 *: 18 *: 19 *: 20 *: 21 *: 22 *: 23 *: EmptyTuple
    println(tuple.size)
    println(tuple.head)
    println(tuple.last)
    def item(n: Int): Int = { print(n); n }
    val ordered = item(1) *: item(2) *: item(3) *: EmptyTuple
    println()
    println(ordered)
