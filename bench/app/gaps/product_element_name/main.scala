// `Product.productElementName` is missing from teq's std: `value productElementName is not a
// member of Row`. scalac prints `name`.
final case class Row(id: Int, name: String)
object Main:
  def main(args: Array[String]): Unit =
    println(Row(1, "n").productElementName(1))
