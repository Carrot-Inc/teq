// The std's `Array.transpose` (scala-library's `ArrayOps.transpose`), as indigo's plugin transposes the rows of
// the data it embeds: the columns as rows, a shorter row leaving its columns shorter, an empty array's none.
final case class Cell(text: String)

@main def run(): Unit =
  val rows = Array(Array(Cell("a"), Cell("b"), Cell("c")), Array(Cell("d"), Cell("e"), Cell("f")))
  val columns = rows.transpose
  println(columns.map(_.map(_.text).mkString(",")).mkString(" | "))
  println(columns.length)
  val ints = Array(Array(1, 2, 3), Array(4, 5), Array(6))
  println(ints.transpose.map(_.mkString("[", " ", "]")).mkString(" "))
  println(Array.empty[Array[String]].transpose.length)
  println(Array(List("x", "y"), List("z", "w")).map(_.toArray).transpose.map(_.toList).toList)
  val sum = rows.transpose.map(_.length).sum
  println(s"cells $sum")
