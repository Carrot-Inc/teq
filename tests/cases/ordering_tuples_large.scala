// scala-library's tuple orderings go up to nine elements, compared element by element.
object Main:
  def main(args: Array[String]): Unit =
    println(summon[Ordering[(Int, Int, Int, Int, Int, Int)]].compare((1, 2, 3, 4, 5, 6), (1, 2, 3, 4, 5, 7)))
    println(List((1, 1, 1, 1, 1, 1, "b"), (1, 1, 1, 1, 1, 1, "a")).sorted)
    println(List((2, 0, 0, 0, 0, 0, 0, 1), (1, 9, 9, 9, 9, 9, 9, 9)).max)
    println(summon[Ordering[(Int, Int, Int, Int, Int, Int, Int, Int, Int)]].compare((1, 1, 1, 1, 1, 1, 1, 1, 2), (1, 1, 1, 1, 1, 1, 1, 1, 1)))
