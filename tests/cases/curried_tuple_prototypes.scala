// Each application clause owns its tuple adaptation, including when both clauses
// have the same formal type and when their formal types differ.
object CurriedTuplePrototypes:
  def numbers(first: (Int, Int))(second: (Int, Int)): Int =
    first._1 * 1000 + first._2 * 100 + second._1 * 10 + second._2

  def labels(first: (Int, Int))(second: (String, String)): String =
    s"${first._1}:${first._2}:${second._1}:${second._2}"

  def main(args: Array[String]): Unit =
    println(numbers(1, 2)(3, 4))
    println(labels(5, 6)("left", "right"))
