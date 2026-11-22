// A Java enum read from its source is sealed by its constants: a sum to a mirror's search, as scalac reads it, whose
// mirror's use is refused, its companion being no value.
object Main {
  def main(args: Array[String]): Unit = {
    val m = summon[scala.deriving.Mirror.Of[E]]
  }
}
