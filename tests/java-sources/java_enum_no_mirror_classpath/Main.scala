// A Java enum of the class path is sealed by its constants for a match, as scalac's class-file reader seals it, and
// is still no sum for a mirror, whether a constant was read first or not.
object Main {
  def main(args: Array[String]): Unit = {
    val a = E.A
    val m = summon[scala.deriving.Mirror.SumOf[E]]
  }
}
