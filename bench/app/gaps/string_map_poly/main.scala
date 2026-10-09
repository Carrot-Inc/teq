// teq types `String.map` as `(Char => Char) => String` only: `type mismatch: found Int, required
// Char` for `"ab".map(_.toInt)`, where scalac's StringOps overload gives `IndexedSeq[Int]` and
// prints `List(97, 98)`.
object Main:
  def main(args: Array[String]): Unit =
    println("ab".map(_.toInt).toList)
